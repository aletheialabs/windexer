'use client';

import React, { useState, useEffect, useRef } from 'react';
import { 
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer,
  PieChart, Pie, Cell
} from 'recharts';
import { FileText, RefreshCw, Download, Layers, Zap } from 'lucide-react';
import LoadingSpinner from './LoadingSpinner';

const RAYDIUM_PROGRAM_ID = 'CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK';
const API_BASE_URL = "http://localhost:3000/api";
const COLORS = ['#0088FE', '#00C49F', '#FFBB28', '#FF8042', '#A569BD', '#5DADE2', '#45B39D'];

interface RaydiumTransaction {
  signature: string;
  blockTime: number;
  success: boolean;
  slot: number;
  fee: number;
  instructions: {
    program_id: string;
    accounts: string[];
    data: string;
  }[];
  timestamp: Date;
  instructionType?: string;
}

interface DailyStats {
  date: string;
  transactions: number;
  successRate: number;
  fees: number;
}

interface InstructionTypeStat {
  name: string;
  value: number;
}

const RaydiumTracker: React.FC = () => {
  const [transactions, setTransactions] = useState<RaydiumTransaction[]>([]);
  const [dailyStats, setDailyStats] = useState<DailyStats[]>([]);
  const [instructionStats, setInstructionStats] = useState<InstructionTypeStat[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [isTracking, setIsTracking] = useState<boolean>(false);
  const [idlData, setIdlData] = useState<any>(null);
  const [idlLoading, setIdlLoading] = useState<boolean>(false);
  const wsRef = useRef<WebSocket | null>(null);
  const localDataRef = useRef<RaydiumTransaction[]>([]);

  // Connect to WebSocket and start tracking
  const startTracking = () => {
    const ws = new WebSocket(`ws://localhost:3000/api/ws/transactions?program=${RAYDIUM_PROGRAM_ID}`);
    
    ws.onopen = () => {
      console.log('Connected to WebSocket');
      setIsTracking(true);
    };
    
    ws.onmessage = (event) => {
      try {
        const txData = JSON.parse(event.data);
        
        // Skip if not related to Raydium
        if (!txData.program_ids.includes(RAYDIUM_PROGRAM_ID) && 
            !txData.accounts.includes(RAYDIUM_PROGRAM_ID)) {
          return;
        }
        
        // Process transaction data
        const processedTx: RaydiumTransaction = {
          signature: txData.signature,
          blockTime: txData.block_time || Date.now() / 1000,
          success: txData.success,
          slot: txData.slot,
          fee: txData.fee,
          instructions: txData.instructions || [],
          timestamp: new Date((txData.block_time || Date.now() / 1000) * 1000),
          instructionType: identifyInstructionType(txData.instructions)
        };
        
        // Add to local state
        localDataRef.current = [processedTx, ...localDataRef.current.slice(0, 99)];
        
        // Update displayed transactions
        setTransactions(prevState => [processedTx, ...prevState.slice(0, 19)]);
        
        // Save to local storage
        localStorage.setItem('raydium_transactions', JSON.stringify(localDataRef.current));
        
        // Update stats
        updateStats();
      } catch (error) {
        console.error('Error processing WebSocket message:', error);
      }
    };
    
    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      setIsTracking(false);
    };
    
    ws.onclose = () => {
      console.log('WebSocket connection closed');
      setIsTracking(false);
    };
    
    wsRef.current = ws;
  };

  // Stop tracking
  const stopTracking = () => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
      setIsTracking(false);
    }
  };

  // Identify the type of instruction
  const identifyInstructionType = (instructions: any[] = []): string => {
    if (!instructions || instructions.length === 0) return 'Unknown';
    
    // Get the first instruction for the Raydium program
    const raydiumInst = instructions.find(inst => inst.program_id === RAYDIUM_PROGRAM_ID);
    
    if (!raydiumInst) return 'Other';
    
    // Try to determine the instruction type from the data 
    // This is a simplification, ideally we would decode the instruction data
    const dataPrefix = raydiumInst.data.substring(0, 8); // First 8 chars might indicate instruction type
    
    // Map of known prefixes - this would need to be expanded based on actual data
    const knownPrefixes: {[key: string]: string} = {
      '01000000': 'Swap',
      '02000000': 'Deposit',
      '03000000': 'Withdraw',
      '04000000': 'Create Position',
      '05000000': 'Create Pool',
      '06000000': 'Close Position',
      '07000000': 'Collect Fees',
      '08000000': 'Update Fees',
    };
    
    return knownPrefixes[dataPrefix] || 'Unknown Instruction';
  };

  // Update statistical data for charts
  const updateStats = () => {
    // Skip if no transactions
    if (localDataRef.current.length === 0) return;
    
    // Process daily stats
    const dailyData: {[key: string]: { transactions: number, successful: number, fees: number }} = {};
    
    localDataRef.current.forEach(tx => {
      const date = new Date(tx.timestamp).toISOString().split('T')[0];
      
      if (!dailyData[date]) {
        dailyData[date] = { transactions: 0, successful: 0, fees: 0 };
      }
      
      dailyData[date].transactions += 1;
      if (tx.success) {
        dailyData[date].successful += 1;
      }
      dailyData[date].fees += tx.fee / 1000000000; // Convert lamports to SOL
    });
    
    const dailyStatsArray = Object.keys(dailyData).map(date => ({
      date,
      transactions: dailyData[date].transactions,
      successRate: dailyData[date].transactions > 0 
        ? (dailyData[date].successful / dailyData[date].transactions) * 100 
        : 0,
      fees: parseFloat(dailyData[date].fees.toFixed(6))
    }));
    
    setDailyStats(dailyStatsArray.sort((a, b) => a.date.localeCompare(b.date)));
    
    // Process instruction type stats
    const typeCounts: {[key: string]: number} = {};
    
    localDataRef.current.forEach(tx => {
      const type = tx.instructionType || 'Unknown';
      typeCounts[type] = (typeCounts[type] || 0) + 1;
    });
    
    const instructionStatsArray = Object.keys(typeCounts).map(type => ({
      name: type,
      value: typeCounts[type]
    }));
    
    setInstructionStats(instructionStatsArray);
  };

  // Export tracked data as JSON
  const exportData = () => {
    const dataStr = JSON.stringify(localDataRef.current, null, 2);
    const dataUri = `data:application/json;charset=utf-8,${encodeURIComponent(dataStr)}`;
    
    const linkElement = document.createElement('a');
    linkElement.setAttribute('href', dataUri);
    linkElement.setAttribute('download', `raydium-transactions-${new Date().toISOString()}.json`);
    document.body.appendChild(linkElement);
    linkElement.click();
    document.body.removeChild(linkElement);
  };

  // Fetch recent program transactions on initial load
  const fetchRecentTransactions = async () => {
    setLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/transactions/program/${RAYDIUM_PROGRAM_ID}?limit=20`);
      
      if (!response.ok) {
        throw new Error('Failed to fetch transactions');
      }
      
      const data = await response.json();
      
      if (data.success && data.data) {
        // Process transactions
        const processedTxs = data.data.map((tx: any) => ({
          signature: tx.signature,
          blockTime: tx.block_time || Date.now() / 1000,
          success: tx.success || tx.err === null,
          slot: tx.slot,
          fee: tx.fee,
          instructions: tx.instructions || [],
          timestamp: new Date((tx.block_time || Date.now() / 1000) * 1000),
          instructionType: identifyInstructionType(tx.instructions)
        }));
        
        setTransactions(processedTxs);
        localDataRef.current = processedTxs;
        
        // Save to local storage
        localStorage.setItem('raydium_transactions', JSON.stringify(processedTxs));
        
        // Update stats
        updateStats();
      }
    } catch (error) {
      console.error('Error fetching transactions:', error);
    } finally {
      setLoading(false);
    }
  };

  // Try to fetch Program IDL data
  const fetchProgramIdl = async () => {
    setIdlLoading(true);
    try {
      // First try Anchor IDL repository using GitHub API as a fallback
      const response = await fetch(`https://api.github.com/repos/project-serum/anchor/contents/examples/solana-program-library/cpi-programs/programs/raydium`);
      
      if (!response.ok) {
        // Try Solana program lookup
        const fallbackResponse = await fetch(`${API_BASE_URL}/account/${RAYDIUM_PROGRAM_ID}`);
        
        if (fallbackResponse.ok) {
          const accountData = await fallbackResponse.json();
          
          // Check if data has idl
          if (accountData.success && accountData.data && accountData.data.data) {
            setIdlData({
              name: "Raydium Concentrated Liquidity",
              description: "Raydium AMM program for concentrated liquidity pools",
              address: RAYDIUM_PROGRAM_ID,
              // This is a placeholder, the real IDL would come from accountData
              instructions: ["Creating pools", "Adding liquidity", "Removing liquidity", "Swapping"]
            });
          }
        }
      } else {
        // Simple placeholder IDL until we can parse the actual one
        setIdlData({
          name: "Raydium Concentrated Liquidity",
          description: "Raydium AMM program for concentrated liquidity pools",
          address: RAYDIUM_PROGRAM_ID,
          instructions: ["Creating pools", "Adding liquidity", "Removing liquidity", "Swapping"]
        });
      }
    } catch (error) {
      console.error('Error fetching IDL:', error);
    } finally {
      setIdlLoading(false);
    }
  };

  // Load saved data from localStorage on component mount
  useEffect(() => {
    const savedData = localStorage.getItem('raydium_transactions');
    
    if (savedData) {
      try {
        const parsedData = JSON.parse(savedData);
        setTransactions(parsedData.slice(0, 20));
        localDataRef.current = parsedData;
        updateStats();
      } catch (error) {
        console.error('Error loading saved data:', error);
      }
    } else {
      fetchRecentTransactions();
    }
    
    fetchProgramIdl();
    
    return () => {
      // Clean up WebSocket on unmount
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-800 dark:text-white">Raydium Concentrated Liquidity Tracker</h2>
        <div className="flex space-x-2">
          {isTracking ? (
            <button 
              onClick={stopTracking}
              className="flex items-center space-x-1 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-md transition-colors"
            >
              <Zap className="h-4 w-4" />
              <span>Stop Tracking</span>
            </button>
          ) : (
            <button 
              onClick={startTracking}
              className="flex items-center space-x-1 bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded-md transition-colors"
            >
              <Zap className="h-4 w-4" />
              <span>Start Tracking</span>
            </button>
          )}
          <button 
            onClick={fetchRecentTransactions}
            className="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors"
          >
            <RefreshCw className="h-4 w-4" />
            <span>Refresh</span>
          </button>
          <button 
            onClick={exportData}
            className="flex items-center space-x-1 bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-md transition-colors"
          >
            <Download className="h-4 w-4" />
            <span>Export Data</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Daily Transaction Chart */}
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg shadow">
          <h3 className="text-lg font-medium mb-4 text-gray-800 dark:text-white">Daily Transactions</h3>
          {dailyStats.length > 0 ? (
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={dailyStats}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" />
                  <YAxis />
                  <Tooltip />
                  <Legend />
                  <Bar dataKey="transactions" fill="#8884d8" name="Transactions" />
                  <Bar dataKey="fees" fill="#82ca9d" name="Fees (SOL)" />
                </BarChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="h-[300px] flex items-center justify-center text-gray-500">
              No transaction data available
            </div>
          )}
        </div>

        {/* Instruction Types Pie Chart */}
        <div className="bg-white dark:bg-gray-800 p-4 rounded-lg shadow">
          <h3 className="text-lg font-medium mb-4 text-gray-800 dark:text-white">Instruction Types</h3>
          {instructionStats.length > 0 ? (
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={instructionStats}
                    cx="50%"
                    cy="50%"
                    labelLine={true}
                    outerRadius={100}
                    fill="#8884d8"
                    dataKey="value"
                    label={({ name, percent }) => `${name}: ${(percent * 100).toFixed(0)}%`}
                  >
                    {instructionStats.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => [`${value} transactions`, 'Count']} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="h-[300px] flex items-center justify-center text-gray-500">
              No instruction data available
            </div>
          )}
        </div>
      </div>

      {/* Program IDL Information */}
      <div className="bg-white dark:bg-gray-800 p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium mb-4 text-gray-800 dark:text-white">Program IDL</h3>
        {idlLoading ? (
          <LoadingSpinner message="Loading IDL data..." />
        ) : idlData ? (
          <div className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Program ID</h4>
                <p className="font-mono break-all dark:text-white">{idlData.address}</p>
              </div>
              <div>
                <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Name</h4>
                <p className="dark:text-white">{idlData.name}</p>
              </div>
            </div>
            
            <div>
              <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Description</h4>
              <p className="dark:text-white">{idlData.description}</p>
            </div>
            
            <div>
              <h4 className="text-sm font-medium text-gray-500 dark:text-gray-400">Instructions</h4>
              <ul className="list-disc list-inside dark:text-white">
                {idlData.instructions.map((inst: string, i: number) => (
                  <li key={i}>{inst}</li>
                ))}
              </ul>
            </div>
          </div>
        ) : (
          <div className="text-gray-500">
            No IDL data available for this program
          </div>
        )}
      </div>

      {/* Recent Transactions Table */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
        <div className="p-4 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 flex justify-between items-center">
          <h3 className="font-semibold text-gray-800 dark:text-white">Recent Transactions</h3>
          <div className="text-sm text-gray-500 dark:text-gray-400">
            {transactions.length} transactions {isTracking && <span className="text-green-500">(Live)</span>}
          </div>
        </div>
        
        {loading ? (
          <LoadingSpinner message="Loading transactions..." />
        ) : transactions.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
              <thead className="bg-gray-50 dark:bg-gray-800">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Signature</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Timestamp</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Instruction Type</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Fee</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                {transactions.map((tx) => (
                  <tr key={tx.signature} className="hover:bg-gray-50 dark:hover:bg-gray-700">
                    <td className="px-4 py-3">
                      <div className="font-mono text-sm truncate text-indigo-600 dark:text-indigo-400 max-w-[180px]">
                        {tx.signature}
                      </div>
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                      {tx.timestamp.toLocaleString()}
                    </td>
                    <td className="px-4 py-3 text-sm">
                      <span className="px-2 py-1 rounded-full bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 text-xs">
                        {tx.instructionType || 'Unknown'}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                      {(tx.fee / 1000000000).toFixed(6)} SOL
                    </td>
                    <td className="px-4 py-3 text-sm">
                      <span className={`px-2 py-1 rounded-full ${
                        tx.success 
                          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' 
                          : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                      } text-xs`}>
                        {tx.success ? 'Success' : 'Failed'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-8 text-center text-gray-500 dark:text-gray-400">
            No transactions found for this program
          </div>
        )}
      </div>
    </div>
  );
};

export default RaydiumTracker; 