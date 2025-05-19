'use client';

import React, { useState, useEffect } from 'react';
import { RefreshCw, Search } from 'lucide-react';
import { BlockData } from '../types';

// Base API URL
const API_BASE_URL = "http://localhost:3000/api";

interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
}

const BlockExplorer: React.FC = () => {
  const [blocks, setBlocks] = useState<BlockData[]>([]);
  const [selectedBlock, setSelectedBlock] = useState<BlockData | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [slotInput, setSlotInput] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchLatestBlocks();
  }, []);

  const fetchLatestBlocks = async () => {
    setLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/blocks/latest`);
      if (!response.ok) throw new Error('Failed to fetch latest blocks');
      
      const data = await response.json();
      if (data.success && data.data) {
        // Handle successful response
        setBlocks(Array.isArray(data.data) ? data.data : [data.data]);
      } else {
        // Handle error in response
        setError(data.error || 'Failed to load blocks data');
        setBlocks([]);
      }
    } catch (err) {
      console.error('Error fetching blocks:', err);
      setError('Failed to load latest blocks. Please try again later.');
      setBlocks([]);
    } finally {
      setLoading(false);
    }
  };

  const fetchBlockBySlot = async () => {
    if (!slotInput || isNaN(parseInt(slotInput))) {
      setError('Please enter a valid slot number');
      return;
    }

    setLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/blocks/${slotInput}`);
      if (!response.ok) {
        if (response.status === 404) {
          throw new Error('Block not found');
        }
        throw new Error('Failed to fetch block');
      }
      
      const data = await response.json();
      if (data.success && data.data) {
        setSelectedBlock(data.data);
        setError(null);
      } else {
        throw new Error(data.error || 'Failed to load block data');
      }
    } catch (err) {
      console.error('Error fetching block by slot:', err);
      setError(err instanceof Error ? err.message : 'Failed to load block');
      setSelectedBlock(null);
    } finally {
      setLoading(false);
    }
  };

  const handleBlockSelect = (block: BlockData) => {
    setSelectedBlock(block);
  };

  // Helper component for the loading state
  const LoadingSpinner: React.FC<{ message?: string }> = ({ message = 'Loading...' }) => {
    return (
      <div className="flex flex-col items-center justify-center py-6">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
        <div className="mt-3 text-gray-600 dark:text-gray-400">{message}</div>
      </div>
    );
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-800 dark:text-white">Block Explorer</h2>
        <button 
          onClick={fetchLatestBlocks}
          className="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors"
        >
          <RefreshCw className="h-4 w-4" />
          <span>Refresh</span>
        </button>
      </div>

      {/* Search by slot */}
      <div className="bg-white dark:bg-gray-800 p-4 rounded-lg shadow">
        <div className="flex space-x-2">
          <div className="flex-1">
            <input
              type="text"
              value={slotInput}
              onChange={(e) => setSlotInput(e.target.value)}
              placeholder="Enter slot number"
              className="w-full border border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white rounded-md px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
            />
          </div>
          <button
            onClick={fetchBlockBySlot}
            className="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors flex items-center"
          >
            <Search className="h-5 w-5 mr-1" />
            Search
          </button>
        </div>
        {error && <div className="mt-2 text-red-600 dark:text-red-400 text-sm">{error}</div>}
      </div>

      {loading && <LoadingSpinner message="Loading block data..." />}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Block List */}
        <div className="lg:col-span-1 bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
            <h3 className="font-semibold text-gray-800 dark:text-white">Latest Blocks</h3>
          </div>
          <div className="p-2 max-h-96 overflow-y-auto">
            {blocks.length > 0 ? (
              <ul className="divide-y divide-gray-200 dark:divide-gray-700">
                {blocks.map((block, index) => (
                  <li key={index}>
                    <button
                      onClick={() => handleBlockSelect(block)}
                      className={`w-full p-3 text-left hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors ${
                        selectedBlock && selectedBlock.slot === block.slot ? 'bg-indigo-50 dark:bg-indigo-900' : ''
                      }`}
                    >
                      <div className="flex justify-between">
                        <span className="font-medium text-indigo-600 dark:text-indigo-400">Slot {block.slot}</span>
                        <span className="text-gray-500 dark:text-gray-400 text-sm">
                          {block.block_time ? new Date(block.block_time * 1000).toLocaleTimeString() : 'Unknown time'}
                        </span>
                      </div>
                      <div className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                        {block.transactions || block.transaction_count || 0} transactions
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="p-4 text-center text-gray-500 dark:text-gray-400">No blocks available</div>
            )}
          </div>
        </div>

        {/* Block Details */}
        <div className="lg:col-span-2 bg-white dark:bg-gray-800 rounded-lg shadow">
          <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
            <h3 className="font-semibold text-gray-800 dark:text-white">Block Details</h3>
          </div>
          <div className="p-4">
            {selectedBlock ? (
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Slot</h4>
                    <p className="font-mono dark:text-white">{selectedBlock.slot}</p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Parent Slot</h4>
                    <p className="font-mono dark:text-white">{selectedBlock.parent_slot || 'N/A'}</p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Timestamp</h4>
                    <p className="dark:text-white">
                      {selectedBlock.block_time 
                        ? new Date(selectedBlock.block_time * 1000).toLocaleString() 
                        : 'N/A'}
                    </p>
                  </div>
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Block Height</h4>
                    <p className="dark:text-white">{selectedBlock.block_height || 'N/A'}</p>
                  </div>
                </div>

                {selectedBlock.hash && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Block Hash</h4>
                    <p className="font-mono text-sm truncate dark:text-white">{selectedBlock.hash || selectedBlock.blockhash}</p>
                  </div>
                )}

                {selectedBlock.parent_hash && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Parent Hash</h4>
                    <p className="font-mono text-sm truncate dark:text-white">{selectedBlock.parent_hash || selectedBlock.previous_blockhash}</p>
                  </div>
                )}

                {selectedBlock.leader && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Leader</h4>
                    <p className="font-mono text-sm truncate dark:text-white">{selectedBlock.leader}</p>
                  </div>
                )}

                <div>
                  <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Transactions</h4>
                  <p className="text-lg font-medium dark:text-white">
                    {selectedBlock.transactions || selectedBlock.transaction_count || 0}
                  </p>
                </div>
              </div>
            ) : (
              <div className="text-center py-10 text-gray-500 dark:text-gray-400">
                Select a block to view details
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default BlockExplorer; 