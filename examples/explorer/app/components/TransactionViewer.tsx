'use client';

import React, { useState, useEffect } from 'react';
import { RefreshCw, Search } from 'lucide-react';
import { TransactionData } from '../types';
import LoadingSpinner from './LoadingSpinner';

// Base API URL
const API_BASE_URL = "http://localhost:3000/api";

const TransactionViewer: React.FC = () => {
  const [transactions, setTransactions] = useState<TransactionData[]>([]);
  const [selectedTx, setSelectedTx] = useState<TransactionData | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [signatureInput, setSignatureInput] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchRecentTransactions();
  }, []);

  const fetchRecentTransactions = async () => {
    setLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/transactions/recent`);
      if (!response.ok) throw new Error('Failed to fetch recent transactions');
      
      const data = await response.json();
      if (data.success && data.data) {
        // Process transactions to ensure success property is correctly set
        const processedTxs = Array.isArray(data.data) 
          ? data.data.map(processTxStatus) 
          : [processTxStatus(data.data)];
        
        setTransactions(processedTxs);
      } else {
        setError(data.error || 'Failed to load transaction data');
        setTransactions([]);
      }
    } catch (err) {
      console.error('Error fetching transactions:', err);
      setError('Failed to load recent transactions. Please try again later.');
      setTransactions([]);
    } finally {
      setLoading(false);
    }
  };

  // Helper function to process transaction status
  const processTxStatus = (tx: TransactionData): TransactionData => {
    // Transaction is successful if err is null or undefined
    return {
      ...tx,
      success: tx.err === null || tx.err === undefined
    };
  };

  const fetchTransactionBySignature = async () => {
    if (!signatureInput) {
      setError('Please enter a transaction signature');
      return;
    }

    setLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/transaction/${signatureInput}`);
      if (!response.ok) {
        if (response.status === 404) {
          throw new Error('Transaction not found');
        }
        throw new Error('Failed to fetch transaction');
      }
      
      const data = await response.json();
      if (data.success && data.data) {
        // Process transaction to ensure success property is correctly set
        setSelectedTx(processTxStatus(data.data));
        setError(null);
      } else {
        throw new Error(data.error || 'Failed to load transaction data');
      }
    } catch (err) {
      console.error('Error fetching transaction:', err);
      setError(err instanceof Error ? err.message : 'Failed to load transaction');
      setSelectedTx(null);
    } finally {
      setLoading(false);
    }
  };

  const handleTxSelect = (tx: TransactionData) => {
    setSelectedTx(tx);
  };

  // Format SOL amount from lamports
  const formatLamports = (lamports?: number) => {
    if (!lamports && lamports !== 0) return 'N/A';
    const sol = lamports / 1000000000;
    return `${sol.toLocaleString(undefined, { minimumFractionDigits: 9, maximumFractionDigits: 9 })} SOL`;
  };

  // Check if transaction is successful
  const isSuccessful = (tx: TransactionData): boolean => {
    return tx.err === null || tx.err === undefined;
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-800 dark:text-white">Transaction Viewer</h2>
        <button 
          onClick={fetchRecentTransactions}
          className="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors"
        >
          <RefreshCw className="h-4 w-4" />
          <span>Refresh</span>
        </button>
      </div>

      {/* Search by signature */}
      <div className="bg-white dark:bg-gray-800 p-4 rounded-lg shadow">
        <div className="flex space-x-2">
          <div className="flex-1">
            <input
              type="text"
              value={signatureInput}
              onChange={(e) => setSignatureInput(e.target.value)}
              placeholder="Enter transaction signature"
              className="w-full border border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white rounded-md px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
            />
          </div>
          <button
            onClick={fetchTransactionBySignature}
            className="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors flex items-center"
          >
            <Search className="h-5 w-5 mr-1" />
            Search
          </button>
        </div>
        {error && <div className="mt-2 text-red-600 dark:text-red-400 text-sm">{error}</div>}
      </div>

      {loading && <LoadingSpinner message="Loading transaction data..." />}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Transaction List */}
        <div className="lg:col-span-1 bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
            <h3 className="font-semibold text-gray-800 dark:text-white">Recent Transactions</h3>
          </div>
          <div className="p-2 max-h-96 overflow-y-auto">
            {transactions.length > 0 ? (
              <ul className="divide-y divide-gray-200 dark:divide-gray-700">
                {transactions.map((tx, index) => (
                  <li key={index}>
                    <button
                      onClick={() => handleTxSelect(tx)}
                      className={`w-full p-3 text-left hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors ${
                        selectedTx && selectedTx.signature === tx.signature ? 'bg-indigo-50 dark:bg-indigo-900' : ''
                      }`}
                    >
                      <div className="font-mono text-sm truncate text-indigo-600 dark:text-indigo-400">
                        {tx.signature || 'Unknown Signature'}
                      </div>
                      <div className="text-sm text-gray-600 dark:text-gray-400 mt-1 flex justify-between">
                        <span>Slot: {tx.slot || 'Unknown'}</span>
                        <span className={isSuccessful(tx) ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}>
                          {isSuccessful(tx) ? 'Success' : 'Failed'}
                        </span>
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="p-4 text-center text-gray-500 dark:text-gray-400">No transactions available</div>
            )}
          </div>
        </div>

        {/* Transaction Details */}
        <div className="lg:col-span-2 bg-white dark:bg-gray-800 rounded-lg shadow">
          <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
            <h3 className="font-semibold text-gray-800 dark:text-white">Transaction Details</h3>
          </div>
          <div className="p-4">
            {selectedTx ? (
              <div className="space-y-4">
                <div>
                  <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Signature</h4>
                  <p className="font-mono break-all dark:text-white">{selectedTx.signature}</p>
                </div>
                
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Slot</h4>
                    <p className="dark:text-white">{selectedTx.slot || 'N/A'}</p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Status</h4>
                    <p className={isSuccessful(selectedTx) ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}>
                      {isSuccessful(selectedTx) ? 'Success' : 'Failed'}
                    </p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Timestamp</h4>
                    <p className="dark:text-white">
                      {selectedTx.block_time 
                        ? new Date(selectedTx.block_time * 1000).toLocaleString() 
                        : 'N/A'}
                    </p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Fee</h4>
                    <p className="dark:text-white">{formatLamports(selectedTx.fee)}</p>
                  </div>
                </div>

                {selectedTx.instructions && selectedTx.instructions.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Instructions</h4>
                    <div className="mt-2 border border-gray-200 dark:border-gray-700 rounded-md">
                      <div className="max-h-64 overflow-y-auto">
                        {selectedTx.instructions.map((instruction, idx) => (
                          <div key={idx} className="p-3 border-b border-gray-200 dark:border-gray-700 last:border-b-0">
                            <div className="font-medium dark:text-white">Instruction {idx + 1}</div>
                            <div className="mt-1 text-sm">
                              <div className="dark:text-white"><span className="text-gray-500 dark:text-gray-400">Program:</span> {instruction.program_id || 'Unknown'}</div>
                              {instruction.accounts && instruction.accounts.length > 0 && (
                                <div className="mt-2">
                                  <div className="text-gray-500 dark:text-gray-400">Accounts:</div>
                                  <ul className="list-disc list-inside pl-2 dark:text-gray-300">
                                    {instruction.accounts.map((account, accIdx) => (
                                      <li key={accIdx} className="truncate font-mono text-xs">
                                        {account}
                                      </li>
                                    ))}
                                  </ul>
                                </div>
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                )}
                
                {selectedTx.logs && selectedTx.logs.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Log Messages</h4>
                    <div className="mt-2 bg-gray-50 dark:bg-gray-900 p-3 rounded-md font-mono text-xs max-h-40 overflow-y-auto">
                      {selectedTx.logs.map((log, idx) => (
                        <div key={idx} className="py-1 dark:text-gray-300">{log}</div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="text-center py-10 text-gray-500 dark:text-gray-400">
                Select a transaction to view details
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default TransactionViewer; 