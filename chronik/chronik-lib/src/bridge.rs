// Copyright (c) 2022 The Bitcoin developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Rust side of the bridge; these structs and functions are exposed to C++.

use std::{
    net::{AddrParseError, IpAddr, SocketAddr},
    sync::Arc,
};

use abc_rust_error::Result;
use bitcoinsuite_core::{
    script::Script,
    tx::{Tx, TxId},
};
use chronik_bridge::{ffi::init_error, util::expect_unique_ptr};
use chronik_db::mem::MempoolTx;
use chronik_http::server::{ChronikServer, ChronikServerParams};
use chronik_indexer::indexer::{ChronikIndexer, ChronikIndexerParams};
use chronik_util::{log, log_chronik};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{
    error::ok_or_abort_node,
    ffi::{self, StartChronikValidationInterface},
};

/// Errors for [`Chronik`] and [`setup_chronik`].
#[derive(Debug, Eq, Error, PartialEq)]
pub enum ChronikError {
    /// Chronik host address failed to parse
    #[error("Invalid Chronik host address {0:?}: {1}")]
    InvalidChronikHost(String, AddrParseError),
}

use self::ChronikError::*;

/// Setup the Chronik bridge. Returns a ChronikIndexer object.
pub fn setup_chronik(
    params: ffi::SetupParams,
    config: &ffi::Config,
    node: &ffi::NodeContext,
) -> bool {
    match try_setup_chronik(params, config, node) {
        Ok(()) => true,
        Err(report) => {
            log_chronik!("{report:?}\n");
            init_error(&report.to_string())
        }
    }
}

fn try_setup_chronik(
    params: ffi::SetupParams,
    config: &ffi::Config,
    node: &ffi::NodeContext,
) -> Result<()> {
    abc_rust_error::install();
    let hosts = params
        .hosts
        .into_iter()
        .map(|host| parse_socket_addr(host, params.default_port))
        .collect::<Result<Vec<_>>>()?;
    log!("Starting Chronik bound to {:?}\n", hosts);
    let bridge = chronik_bridge::ffi::make_bridge(config, node);
    let bridge_ref = expect_unique_ptr("make_bridge", &bridge);
    let mut indexer = ChronikIndexer::setup(ChronikIndexerParams {
        datadir_net: params.datadir_net.into(),
        wipe_db: params.wipe_db,
        fn_compress_script: compress_script,
    })?;
    indexer.resync_indexer(bridge_ref)?;
    let indexer = Arc::new(RwLock::new(indexer));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let server = runtime.block_on({
        let indexer = Arc::clone(&indexer);
        async move {
            // try_bind requires a Runtime
            ChronikServer::setup(ChronikServerParams { hosts, indexer })
        }
    })?;
    runtime.spawn(async move {
        ok_or_abort_node("ChronikServer::serve", server.serve().await);
    });
    let chronik = Box::new(Chronik {
        bridge: Arc::new(bridge),
        indexer,
        _runtime: runtime,
    });
    StartChronikValidationInterface(node, chronik);
    Ok(())
}

fn parse_socket_addr(host: String, default_port: u16) -> Result<SocketAddr> {
    if let Ok(addr) = host.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let ip_addr = host
        .parse::<IpAddr>()
        .map_err(|err| InvalidChronikHost(host, err))?;
    Ok(SocketAddr::new(ip_addr, default_port))
}

fn compress_script(script: &Script) -> Vec<u8> {
    chronik_bridge::ffi::compress_script(script.as_ref())
}

/// Contains all db, runtime, tpc, etc. handles needed by Chronik.
/// This makes it so when this struct is dropped, all handles are relased
/// cleanly.
pub struct Chronik {
    bridge: Arc<cxx::UniquePtr<ffi::ChronikBridge>>,
    indexer: Arc<RwLock<ChronikIndexer>>,
    // Having this here ensures HTTP server, outstanding requests etc. will get
    // stopped when `Chronik` is dropped.
    _runtime: tokio::runtime::Runtime,
}

impl Chronik {
    /// Tx added to the bitcoind mempool
    pub fn handle_tx_added_to_mempool(
        &self,
        ptx: &ffi::CTransaction,
        time_first_seen: i64,
    ) {
        ok_or_abort_node(
            "handle_tx_added_to_mempool",
            self.add_tx_to_mempool(ptx, time_first_seen),
        );
    }

    /// Tx removed from the bitcoind mempool
    pub fn handle_tx_removed_from_mempool(&self, txid: [u8; 32]) {
        let mut indexer = self.indexer.blocking_write();
        let txid = TxId::from(txid);
        ok_or_abort_node(
            "handle_tx_removed_from_mempool",
            indexer.handle_tx_removed_from_mempool(txid),
        );
        log_chronik!("Chronik: transaction {} removed from mempool\n", txid);
    }

    /// Block connected to the longest chain
    pub fn handle_block_connected(
        &self,
        block: &ffi::CBlock,
        bindex: &ffi::CBlockIndex,
    ) {
        ok_or_abort_node(
            "handle_block_connected",
            self.connect_block(block, bindex),
        );
    }

    /// Block disconnected from the longest chain
    pub fn handle_block_disconnected(
        &self,
        block: &ffi::CBlock,
        bindex: &ffi::CBlockIndex,
    ) {
        ok_or_abort_node(
            "handle_block_disconnected",
            self.disconnect_block(block, bindex),
        );
    }

    /// Block finalized with Avalanche
    pub fn handle_block_finalized(&self, bindex: &ffi::CBlockIndex) {
        ok_or_abort_node("handle_block_finalized", self.finalize_block(bindex));
    }

    fn add_tx_to_mempool(
        &self,
        ptx: &ffi::CTransaction,
        time_first_seen: i64,
    ) -> Result<()> {
        let mut indexer = self.indexer.blocking_write();
        let tx = self.bridge.bridge_tx(ptx)?;
        let txid = TxId::from(tx.txid);
        indexer.handle_tx_added_to_mempool(MempoolTx {
            tx: Tx::from(tx),
            time_first_seen,
        })?;
        log_chronik!("Chronik: transaction {} added to mempool\n", txid);
        Ok(())
    }

    fn connect_block(
        &self,
        block: &ffi::CBlock,
        bindex: &ffi::CBlockIndex,
    ) -> Result<()> {
        let mut indexer = self.indexer.blocking_write();
        let block = indexer.make_chronik_block(block, bindex)?;
        let block_hash = block.db_block.hash.clone();
        let num_txs = block.block_txs.txs.len();
        indexer.handle_block_connected(block)?;
        log_chronik!(
            "Chronik: block {} connected with {} txs\n",
            block_hash,
            num_txs,
        );
        Ok(())
    }

    fn disconnect_block(
        &self,
        block: &ffi::CBlock,
        bindex: &ffi::CBlockIndex,
    ) -> Result<()> {
        let mut indexer = self.indexer.blocking_write();
        let block = indexer.make_chronik_block(block, bindex)?;
        let block_hash = block.db_block.hash.clone();
        let num_txs = block.block_txs.txs.len();
        indexer.handle_block_disconnected(block)?;
        log_chronik!(
            "Chronik: block {} disconnected with {} txs\n",
            block_hash,
            num_txs,
        );
        Ok(())
    }

    fn finalize_block(&self, bindex: &ffi::CBlockIndex) -> Result<()> {
        let block = self.bridge.load_block(bindex)?;
        let block_ref = expect_unique_ptr("load_block", &block);
        let mut indexer = self.indexer.blocking_write();
        let block = indexer.make_chronik_block(block_ref, bindex)?;
        let block_hash = block.db_block.hash.clone();
        let num_txs = block.block_txs.txs.len();
        indexer.handle_block_finalized(block)?;
        log_chronik!(
            "Chronik: block {} finalized with {} txs\n",
            block_hash,
            num_txs,
        );
        Ok(())
    }
}

impl std::fmt::Debug for Chronik {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chronik {{ .. }}")
    }
}
