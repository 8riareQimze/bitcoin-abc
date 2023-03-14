const assert = require('assert');
const config = require('../config');
const { getUnprocessedTxHistory } = require('../chronik');
const {
    allTxHistoryFromChronik,
    unconfirmedTxs,
    unconfirmedTxsAfterConfirmation,
} = require('./mocks/txHistoryMocks');

// todo make txsperpage a param and test different values
describe('alias-server chronik.js', () => {
    it('getUnprocessedTxHistory correctly recognizes when no unprocessed transactions are available', async () => {
        const processedTxs = allTxHistoryFromChronik;
        const processedBlockheight = processedTxs[0].block.height;
        const processedTxCount = processedTxs.length;
        const unprocessedTxs = [];
        const allTxHistory = unprocessedTxs.concat(processedTxs);

        const txHistoryFirstPageResponse = {
            txs: allTxHistory.slice(0, config.txHistoryPageSize),
            numPages: Math.ceil(allTxHistory.length / config.txHistoryPageSize),
        };
        const optionalMocks = {
            txHistoryFirstPageResponse,
            remainingTxHistoryPageResponses: [],
        };

        const result = await getUnprocessedTxHistory(
            config.aliasConstants.registrationHash160,
            processedBlockheight,
            processedTxCount,
            optionalMocks,
        );
        const expectedResult = {
            maxTxs: undefined,
            maxUnprocessedTxCount: undefined,
            numPagesToFetch: undefined,
            alreadyHaveAllPotentiallyUnprocessedTxs: true,
            unprocessedTxs: unprocessedTxs,
        };

        assert.deepEqual(result, expectedResult);
    });
    it(`getUnprocessedTxHistory correctly recognizes when 11 unprocessed unconfirmed transactions are available with a txHistoryPageSize=${config.txHistoryPageSize}`, async () => {
        const processedTxs = allTxHistoryFromChronik;
        const processedBlockheight = processedTxs[0].block.height;
        const processedTxCount = processedTxs.length;
        const unprocessedTxs = unconfirmedTxs;
        const allTxHistory = unprocessedTxs.concat(processedTxs);

        const numPages = Math.ceil(
            allTxHistory.length / config.txHistoryPageSize,
        );

        const txHistoryFirstPageResponse = {
            txs: allTxHistory.slice(0, config.txHistoryPageSize),
            numPages,
        };
        const optionalMocks = {
            txHistoryFirstPageResponse,
            remainingTxHistoryPageResponses: [],
        };

        const result = await getUnprocessedTxHistory(
            config.aliasConstants.registrationHash160,
            processedBlockheight,
            processedTxCount,
            optionalMocks,
        );
        const expectedResult = {
            maxTxs: undefined,
            maxUnprocessedTxCount: undefined,
            numPagesToFetch: undefined,
            alreadyHaveAllPotentiallyUnprocessedTxs: true,
            unprocessedTxs: unprocessedTxs,
        };

        assert.deepEqual(result, expectedResult);
    });
    it(`getUnprocessedTxHistory correctly recognizes when 11 unprocessed confirmed transactions are available with a txHistoryPageSize=${config.txHistoryPageSize}`, async () => {
        const processedTxs = allTxHistoryFromChronik;
        const processedBlockheight = processedTxs[0].block.height;
        const processedTxCount = processedTxs.length;
        const unprocessedTxs = unconfirmedTxsAfterConfirmation;
        const allTxHistory = unprocessedTxs.concat(processedTxs);

        const numPages = Math.ceil(
            allTxHistory.length / config.txHistoryPageSize,
        );

        const txHistoryFirstPageResponse = {
            txs: allTxHistory.slice(0, config.txHistoryPageSize),
            numPages,
        };
        const optionalMocks = {
            txHistoryFirstPageResponse,
            remainingTxHistoryPageResponses: [],
        };

        const result = await getUnprocessedTxHistory(
            config.aliasConstants.registrationHash160,
            processedBlockheight,
            processedTxCount,
            optionalMocks,
        );
        const expectedResult = {
            maxTxs: undefined,
            maxUnprocessedTxCount: undefined,
            numPagesToFetch: undefined,
            alreadyHaveAllPotentiallyUnprocessedTxs: true,
            unprocessedTxs: unprocessedTxs,
        };

        assert.deepEqual(result, expectedResult);
    });
    it(`getUnprocessedTxHistory correctly fetches and parses required tx history with a txHistoryPageSize=${config.txHistoryPageSize}`, async () => {
        // Note: txs are processed by blockheight. So, test will only work if you look at batches of txs from different blocks
        // There is an operating assumption here that chronik tx history will always return all confirmed txs at a given address

        const blockheightDeltaTxCounts = [];
        for (let i = 0; i < allTxHistoryFromChronik.length - 1; i += 1) {
            // Find the blockheight cutoffs
            const thisTx = allTxHistoryFromChronik[i];
            const nextTx = allTxHistoryFromChronik[i + 1];
            let thisTxBlockheight, nextTxBlockheight;
            thisTxBlockheight =
                typeof thisTx.block === 'undefined'
                    ? config.unconfirmedBlockheight
                    : thisTx.block.height;
            nextTxBlockheight =
                typeof nextTx.block === 'undefined'
                    ? config.unconfirmedBlockheight
                    : nextTx.block.height;

            // If the next blockheight is different, then an acceptable number
            // of unprocessedTxs to exclude from a given set of chronikTxHistory is (i + 1)
            if (thisTxBlockheight !== nextTxBlockheight) {
                blockheightDeltaTxCounts.push(i + 1);
            }
        }

        // Iterate over all block cutoff amounts of unprocessedTxs to test
        for (let i = 0; i < blockheightDeltaTxCounts; i += 1) {
            const desiredUnprocessedTxs = blockheightDeltaTxCounts[i];

            // Create an array processedTxs that is equivalent to allTxHistoryFromChronik less the most recent desiredUnprocessedTxs
            // i.e. remove the first desiredUnprocessedTxs entries
            const processedTxs = allTxHistoryFromChronik.slice(
                desiredUnprocessedTxs,
            );
            const processedBlockheight = processedTxs[0].block.height;
            const processedTxCount = processedTxs.length;
            // Create an array unprocessedTxs that is equivalent to the most recent desiredUnprocessedTxs txs in allTxHistoryFromChronik
            // i.e. take only the first desiredUnprocessedTxs entries
            const unprocessedTxs = allTxHistoryFromChronik.slice(
                0,
                desiredUnprocessedTxs,
            );
            console.log(`processedTxs.length`, processedTxs.length);
            console.log(`unprocessedTxs.length`, unprocessedTxs.length);
            console.log(
                `processedTxs.length + unprocessedTxs.length`,
                processedTxs.length + unprocessedTxs.length,
            );
            const allTxHistory = allTxHistoryFromChronik;

            const numPages = Math.ceil(
                allTxHistory.length / config.txHistoryPageSize,
            );

            const txHistoryFirstPageTxs = allTxHistory.slice(
                0,
                config.txHistoryPageSize,
            );
            const oldestTxOnFirstPage =
                txHistoryFirstPageTxs[txHistoryFirstPageTxs.length - 1];

            let alreadyHaveAllPotentiallyUnprocessedTxs = false;
            if (typeof oldestTxOnFirstPage.block === 'undefined') {
                alreadyHaveAllPotentiallyUnprocessedTxs = false;
            } else {
                // If the oldest tx on the first page hasa blockheight that has already been processed
                if (oldestTxOnFirstPage.block.height <= processedBlockheight) {
                    // Then you have enough txs with this one call
                    alreadyHaveAllPotentiallyUnprocessedTxs = true;
                }
            }

            const txHistoryFirstPageResponse = {
                txs: txHistoryFirstPageTxs,
                numPages,
            };
            // Calculate these values as in the function
            let maxTxs, maxUnprocessedTxCount, numPagesToFetch;
            let remainingTxHistoryPageResponses = [];
            if (!alreadyHaveAllPotentiallyUnprocessedTxs) {
                maxTxs = config.txHistoryPageSize * numPages;
                maxUnprocessedTxCount = maxTxs - processedTxCount;
                numPagesToFetch = Math.ceil(
                    maxUnprocessedTxCount / config.txHistoryPageSize,
                );
                // Create this mock chronik response as in the function
                for (let i = 1; i < numPagesToFetch; i += 1) {
                    // each page will have config.txHistoryPageSize txs
                    // txs will be ordered most recent to oldest
                    const txs = unprocessedTxs.slice(
                        i * config.txHistoryPageSize,
                        (i + 1) * config.txHistoryPageSize,
                    );
                    remainingTxHistoryPageResponses.push({ txs, numPages });
                }
            }

            const optionalMocks = {
                txHistoryFirstPageResponse,
                remainingTxHistoryPageResponses,
            };

            const result = await getUnprocessedTxHistory(
                config.aliasConstants.registrationHash160,
                processedBlockheight,
                processedTxCount,
                optionalMocks,
            );
            const expectedResult = {
                maxTxs,
                maxUnprocessedTxCount,
                numPagesToFetch,
                alreadyHaveAllPotentiallyUnprocessedTxs,
                unprocessedTxs: unprocessedTxs,
            };

            assert.deepEqual(result, expectedResult);
        }
    });
});
