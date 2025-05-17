'use client';

import React, { useState, useEffect } from 'react';
import { Search, RefreshCw, Plus, Trash, Eye, Download, Filter, Wallet, FileText, ArrowDown, ArrowUp, Calendar } from 'lucide-react';

// Base API URL
const API_BASE_URL = "http://localhost:3000/api";

type EntityType = 'account' | 'transaction' | 'token';
type SortDirection = 'asc' | 'desc';

interface WatchedEntity {
  id: string;
  type: EntityType;
  pubkey: string;
  added: Date;
  name?: string;
}

interface EntityData {
  [key: string]: any; // Will hold the fetched data for each entity
}

interface TokenBalance {
  mint: string;
  symbol: string;
  name: string;
  balance: number;
  price?: number;
  value?: number;
}

export default function MultiIndex() {
  // State for watched entities
  const [watchedEntities, setWatchedEntities] = useState<WatchedEntity[]>([]);
  const [entityData, setEntityData] = useState<EntityData>({});
  const [loading, setLoading] = useState<{[key: string]: boolean}>({});
  
  // State for adding new entity
  const [newEntityInput, setNewEntityInput] = useState("");
  const [selectedType, setSelectedType] = useState<EntityType>('account');
  const [customName, setCustomName] = useState("");
  
  // State for comparison view
  const [compareMode, setCompareMode] = useState(false);
  const [sortField, setSortField] = useState<string | null>(null);
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

  // Load saved entities from localStorage on first render
  useEffect(() => {
    const savedEntities = localStorage.getItem('watchedEntities');
    if (savedEntities) {
      try {
        const parsed = JSON.parse(savedEntities);
        // Convert string dates back to Date objects
        const entitiesWithDateObjects = parsed.map((entity: any) => ({
          ...entity,
          added: new Date(entity.added)
        }));
        setWatchedEntities(entitiesWithDateObjects);
      } catch (e) {
        console.error("Failed to parse saved entities:", e);
      }
    }
  }, []);

  // Save entities to localStorage when they change
  useEffect(() => {
    localStorage.setItem('watchedEntities', JSON.stringify(watchedEntities));
  }, [watchedEntities]);

  // Fetch data for all watched entities
  useEffect(() => {
    const fetchAllEntityData = async () => {
      for (const entity of watchedEntities) {
        await fetchEntityData(entity.id);
      }
    };
    
    fetchAllEntityData();
  }, [watchedEntities]);

  // Function to add a new entity to watch
  const addEntity = () => {
    if (!newEntityInput.trim()) return;
    
    const newEntity: WatchedEntity = {
      id: `${selectedType}-${newEntityInput}-${Date.now()}`,
      type: selectedType,
      pubkey: newEntityInput,
      added: new Date(),
      name: customName || undefined
    };
    
    setWatchedEntities(prev => [...prev, newEntity]);
    setNewEntityInput("");
    setCustomName("");
    
    // Fetch data for the new entity
    fetchEntityData(newEntity.id);
  };

  // Function to remove entity from watch list
  const removeEntity = (id: string) => {
    setWatchedEntities(prev => prev.filter(entity => entity.id !== id));
    
    // Also remove its data
    setEntityData(prev => {
      const newData = {...prev};
      delete newData[id];
      return newData;
    });
  };

  // Function to fetch data for a specific entity
  const fetchEntityData = async (entityId: string) => {
    const entity = watchedEntities.find(e => e.id === entityId);
    if (!entity) return;
    
    setLoading(prev => ({...prev, [entityId]: true}));
    
    try {
      let endpoint = '';
      switch (entity.type) {
        case 'account':
          endpoint = `/account/${entity.pubkey}`;
          break;
        case 'transaction':
          endpoint = `/transaction/${entity.pubkey}`;
          break;
        case 'token':
          endpoint = `/account/${entity.pubkey}/tokens`;
          break;
      }
      
      const response = await fetch(`${API_BASE_URL}${endpoint}`);
      if (!response.ok) throw new Error(`HTTP error ${response.status}`);
      
      const data = await response.json();
      
      setEntityData(prev => ({
        ...prev,
        [entityId]: {
          fetched: new Date(),
          data: data.success ? data.data : data
        }
      }));
    } catch (error) {
      console.error(`Error fetching data for ${entity.type} ${entity.pubkey}:`, error);
      setEntityData(prev => ({
        ...prev,
        [entityId]: {
          fetched: new Date(),
          error: true,
          message: error instanceof Error ? error.message : 'Unknown error'
        }
      }));
    } finally {
      setLoading(prev => ({...prev, [entityId]: false}));
    }
  };

  // Function to refresh data for all entities
  const refreshAll = async () => {
    for (const entity of watchedEntities) {
      await fetchEntityData(entity.id);
    }
  };

  // Function to refresh data for a specific entity
  const refreshEntity = (entityId: string) => {
    fetchEntityData(entityId);
  };

  // Function to export data as JSON
  const exportData = () => {
    const dataToExport = {
      watchedEntities,
      data: entityData,
      exportedAt: new Date().toISOString()
    };
    
    const dataStr = JSON.stringify(dataToExport, null, 2);
    const dataUri = `data:application/json;charset=utf-8,${encodeURIComponent(dataStr)}`;
    
    const linkElement = document.createElement('a');
    linkElement.setAttribute('href', dataUri);
    linkElement.setAttribute('download', `windexer-export-${new Date().toISOString()}.json`);
    document.body.appendChild(linkElement);
    linkElement.click();
    document.body.removeChild(linkElement);
  };

  // Function to sort entities
  const sortEntities = (field: string) => {
    // If clicking the same field, toggle direction
    if (field === sortField) {
      setSortDirection(prev => prev === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc'); // Default to descending for new field
    }
  };

  // Get sorted entities
  const getSortedEntities = () => {
    if (!sortField) return watchedEntities;
    
    return [...watchedEntities].sort((a, b) => {
      if (sortField === 'added') {
        return sortDirection === 'asc' 
          ? a.added.getTime() - b.added.getTime()
          : b.added.getTime() - a.added.getTime();
      } else if (sortField === 'name') {
        const aName = a.name || a.pubkey;
        const bName = b.name || b.pubkey;
        return sortDirection === 'asc'
          ? aName.localeCompare(bName)
          : bName.localeCompare(aName);
      } else if (sortField === 'type') {
        return sortDirection === 'asc'
          ? a.type.localeCompare(b.type)
          : b.type.localeCompare(a.type);
      }
      return 0;
    });
  };

  // Function to render account data
  const renderAccountData = (data: any) => {
    if (!data) return <div className="text-gray-400">No data available</div>;
    
    return (
      <div className="space-y-2">
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-gray-50 p-2 rounded">
            <div className="text-xs text-gray-500">Balance</div>
            <div className="font-medium">{formatLamports(data.lamports)}</div>
          </div>
          <div className="bg-gray-50 p-2 rounded">
            <div className="text-xs text-gray-500">Updated</div>
            <div className="font-medium">{data.updated_at ? new Date(data.updated_at * 1000).toLocaleString() : 'N/A'}</div>
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Owner</div>
          <div className="font-mono text-xs truncate">{data.owner || 'N/A'}</div>
        </div>
      </div>
    );
  };

  // Function to render transaction data
  const renderTransactionData = (data: any) => {
    if (!data) return <div className="text-gray-400">No data available</div>;
    
    return (
      <div className="space-y-2">
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-gray-50 p-2 rounded">
            <div className="text-xs text-gray-500">Slot</div>
            <div className="font-medium">{data.slot || 'N/A'}</div>
          </div>
          <div className="bg-gray-50 p-2 rounded">
            <div className="text-xs text-gray-500">Status</div>
            <div className={`font-medium ${data.err ? 'text-red-600' : 'text-green-600'}`}>
              {data.err ? 'Failed' : 'Success'}
            </div>
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Time</div>
          <div className="font-medium">
            {data.block_time ? new Date(data.block_time * 1000).toLocaleString() : 'N/A'}
          </div>
        </div>
        <div>
          <div className="text-xs text-gray-500">Fee</div>
          <div className="font-medium">{formatLamports(data.fee)}</div>
        </div>
      </div>
    );
  };

  // Function to render token data
  const renderTokenData = (data: any) => {
    if (!data || !Array.isArray(data)) return <div className="text-gray-400">No token data available</div>;
    
    return (
      <div className="space-y-2">
        <div className="text-sm font-medium">{data.length} tokens found</div>
        <div className="max-h-40 overflow-y-auto">
          <table className="min-w-full divide-y divide-gray-200 text-sm">
            <thead className="bg-gray-50">
              <tr>
                <th className="p-2 text-left text-xs font-medium text-gray-500">Token</th>
                <th className="p-2 text-right text-xs font-medium text-gray-500">Balance</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200">
              {data.map((token, index) => (
                <tr key={index} className="hover:bg-gray-50">
                  <td className="p-2">
                    {token.name || token.symbol || 'Unknown'}
                  </td>
                  <td className="p-2 text-right font-medium">
                    {token.balance?.toLocaleString() || '0'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  // Helper function to format lamports as SOL
  const formatLamports = (lamports: number) => {
    if (lamports === undefined || lamports === null) return 'N/A';
    const sol = lamports / 1000000000;
    return `${sol.toLocaleString(undefined, { minimumFractionDigits: 4, maximumFractionDigits: 9 })} SOL`;
  };

  // Helper function to truncate public keys
  const truncateKey = (key: string) => {
    if (!key) return 'Unknown';
    return `${key.slice(0, 4)}...${key.slice(-4)}`;
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-800">Multi-Index Dashboard</h2>
        <div className="flex space-x-2">
          <button 
            onClick={() => setCompareMode(!compareMode)}
            className={`px-4 py-2 rounded-md transition-colors ${
              compareMode ? 'bg-indigo-100 text-indigo-700' : 'bg-gray-100 text-gray-700'
            }`}
          >
            {compareMode ? 'Detail View' : 'Compare View'}
          </button>
          <button 
            onClick={refreshAll}
            className="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md transition-colors"
          >
            <RefreshCw className="h-4 w-4" />
            <span>Refresh All</span>
          </button>
          <button 
            onClick={exportData}
            className="flex items-center space-x-1 bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded-md transition-colors"
          >
            <Download className="h-4 w-4" />
            <span>Export</span>
          </button>
        </div>
      </div>

      {/* Add new entity form */}
      <div className="bg-white p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-800 mb-4">Add Entity to Watch</h3>
        <div className="flex flex-wrap gap-3">
          <div className="flex-1 min-w-[250px]">
            <div className="mb-2 text-sm text-gray-600">Public Key or Signature</div>
            <input
              type="text"
              value={newEntityInput}
              onChange={(e) => setNewEntityInput(e.target.value)}
              placeholder="Enter pubkey or transaction signature"
              className="w-full border border-gray-300 rounded-md px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
            />
          </div>
          
          <div className="w-40">
            <div className="mb-2 text-sm text-gray-600">Type</div>
            <select
              value={selectedType}
              onChange={(e) => setSelectedType(e.target.value as EntityType)}
              className="w-full border border-gray-300 rounded-md px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
            >
              <option value="account">Account</option>
              <option value="transaction">Transaction</option>
              <option value="token">Token Balance</option>
            </select>
          </div>
          
          <div className="flex-1 min-w-[200px]">
            <div className="mb-2 text-sm text-gray-600">Custom Name (Optional)</div>
            <input
              type="text"
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              placeholder="Enter a friendly name"
              className="w-full border border-gray-300 rounded-md px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500"
            />
          </div>
          
          <div className="flex items-end">
            <button
              onClick={addEntity}
              disabled={!newEntityInput.trim()}
              className="flex items-center space-x-1 bg-indigo-600 hover:bg-indigo-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-md transition-colors h-10"
            >
              <Plus className="h-4 w-4" />
              <span>Add</span>
            </button>
          </div>
        </div>
      </div>

      {/* Entity sorting controls */}
      {watchedEntities.length > 0 && (
        <div className="bg-white p-3 rounded-lg shadow flex justify-between items-center">
          <div className="text-sm text-gray-500">
            {watchedEntities.length} {watchedEntities.length === 1 ? 'entity' : 'entities'} being tracked
          </div>
          <div className="flex space-x-2">
            <button
              onClick={() => sortEntities('type')}
              className={`px-3 py-1.5 rounded text-sm flex items-center ${
                sortField === 'type' ? 'bg-indigo-100 text-indigo-700' : 'bg-gray-100 text-gray-600'
              }`}
            >
              <span>Type</span>
              {sortField === 'type' && (
                sortDirection === 'asc' ? <ArrowUp className="h-3 w-3 ml-1" /> : <ArrowDown className="h-3 w-3 ml-1" />
              )}
            </button>
            <button
              onClick={() => sortEntities('name')}
              className={`px-3 py-1.5 rounded text-sm flex items-center ${
                sortField === 'name' ? 'bg-indigo-100 text-indigo-700' : 'bg-gray-100 text-gray-600'
              }`}
            >
              <span>Name</span>
              {sortField === 'name' && (
                sortDirection === 'asc' ? <ArrowUp className="h-3 w-3 ml-1" /> : <ArrowDown className="h-3 w-3 ml-1" />
              )}
            </button>
            <button
              onClick={() => sortEntities('added')}
              className={`px-3 py-1.5 rounded text-sm flex items-center ${
                sortField === 'added' ? 'bg-indigo-100 text-indigo-700' : 'bg-gray-100 text-gray-600'
              }`}
            >
              <span>Date Added</span>
              {sortField === 'added' && (
                sortDirection === 'asc' ? <ArrowUp className="h-3 w-3 ml-1" /> : <ArrowDown className="h-3 w-3 ml-1" />
              )}
            </button>
          </div>
        </div>
      )}

      {/* List of entities - different layout based on compare mode */}
      {watchedEntities.length === 0 ? (
        <div className="bg-white p-10 rounded-lg shadow text-center">
          <div className="text-gray-500 mb-2">No entities being tracked</div>
          <div className="text-sm text-gray-400">Add an account, transaction, or token above to get started</div>
        </div>
      ) : compareMode ? (
        // Compare mode view (table)
        <div className="bg-white rounded-lg shadow overflow-hidden">
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Type</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Public Key</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Key Data</th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Added</th>
                  <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200">
                {getSortedEntities().map(entity => {
                  const entityDataObj = entityData[entity.id];
                  const data = entityDataObj?.data;
                  const isLoading = loading[entity.id];
                  const hasError = entityDataObj?.error;
                  
                  // Extract key data based on entity type
                  let keyData = 'No data';
                  
                  if (data) {
                    if (entity.type === 'account' && data.lamports) {
                      keyData = formatLamports(data.lamports);
                    } else if (entity.type === 'transaction' && data.slot) {
                      keyData = `Slot: ${data.slot}`;
                    } else if (entity.type === 'token' && Array.isArray(data)) {
                      keyData = `${data.length} tokens`;
                    }
                  }
                  
                  return (
                    <tr key={entity.id} className="hover:bg-gray-50">
                      <td className="px-4 py-4 text-sm">
                        {entity.name || truncateKey(entity.pubkey)}
                      </td>
                      <td className="px-4 py-4">
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                          entity.type === 'account' ? 'bg-blue-100 text-blue-800' :
                          entity.type === 'transaction' ? 'bg-purple-100 text-purple-800' :
                          'bg-green-100 text-green-800'
                        }`}>
                          {entity.type === 'account' && <Wallet className="h-3 w-3 mr-1" />}
                          {entity.type === 'transaction' && <FileText className="h-3 w-3 mr-1" />}
                          {entity.type === 'token' && <Eye className="h-3 w-3 mr-1" />}
                          {entity.type}
                        </span>
                      </td>
                      <td className="px-4 py-4 font-mono text-xs truncate max-w-xs">
                        {entity.pubkey}
                      </td>
                      <td className="px-4 py-4 text-sm">
                        {isLoading ? (
                          <span className="text-gray-400">Loading...</span>
                        ) : hasError ? (
                          <span className="text-red-500">Error loading data</span>
                        ) : (
                          keyData
                        )}
                      </td>
                      <td className="px-4 py-4 text-sm text-gray-500">
                        {entity.added.toLocaleDateString()}
                      </td>
                      <td className="px-4 py-4 text-right">
                        <div className="flex justify-end space-x-2">
                          <button 
                            onClick={() => refreshEntity(entity.id)}
                            className="text-gray-500 hover:text-indigo-600"
                            disabled={isLoading}
                          >
                            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin text-indigo-400' : ''}`} />
                          </button>
                          <button 
                            onClick={() => removeEntity(entity.id)}
                            className="text-gray-500 hover:text-red-600"
                          >
                            <Trash className="h-4 w-4" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        // Detail mode view (cards)
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {getSortedEntities().map(entity => {
            const entityDataObj = entityData[entity.id];
            const data = entityDataObj?.data;
            const isLoading = loading[entity.id];
            const hasError = entityDataObj?.error;
            const lastFetched = entityDataObj?.fetched;
            
            return (
              <div key={entity.id} className="bg-white rounded-lg shadow overflow-hidden">
                <div className="p-4 border-b border-gray-200">
                  <div className="flex justify-between items-start">
                    <div>
                      <h3 className="font-medium text-gray-800">
                        {entity.name || truncateKey(entity.pubkey)}
                      </h3>
                      <div className="flex items-center space-x-2 mt-1">
                        <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                          entity.type === 'account' ? 'bg-blue-100 text-blue-800' :
                          entity.type === 'transaction' ? 'bg-purple-100 text-purple-800' :
                          'bg-green-100 text-green-800'
                        }`}>
                          {entity.type}
                        </span>
                        <span className="text-xs text-gray-500">
                          Added {entity.added.toLocaleDateString()}
                        </span>
                      </div>
                    </div>
                    <div className="flex space-x-1">
                      <button 
                        onClick={() => refreshEntity(entity.id)}
                        className="text-gray-400 hover:text-indigo-600 p-1"
                        disabled={isLoading}
                      >
                        <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin text-indigo-400' : ''}`} />
                      </button>
                      <button 
                        onClick={() => removeEntity(entity.id)}
                        className="text-gray-400 hover:text-red-600 p-1"
                      >
                        <Trash className="h-4 w-4" />
                      </button>
                    </div>
                  </div>
                  <div className="mt-1 font-mono text-xs text-gray-500 truncate">
                    {entity.pubkey}
                  </div>
                </div>
                
                <div className="p-4">
                  {isLoading ? (
                    <div className="h-24 flex items-center justify-center">
                      <div className="animate-pulse flex space-x-2 items-center">
                        <div className="h-4 w-4 bg-indigo-400 rounded-full animate-bounce"></div>
                        <div className="text-gray-400">Loading data...</div>
                      </div>
                    </div>
                  ) : hasError ? (
                    <div className="p-4 text-center text-red-500">
                      <div>Error loading data</div>
                      <div className="text-xs mt-1">{entityDataObj?.message}</div>
                    </div>
                  ) : !data ? (
                    <div className="h-24 flex items-center justify-center text-gray-400">
                      No data available
                    </div>
                  ) : (
                    <>
                      {entity.type === 'account' && renderAccountData(data)}
                      {entity.type === 'transaction' && renderTransactionData(data)}
                      {entity.type === 'token' && renderTokenData(data)}
                    </>
                  )}
                </div>
                
                {lastFetched && (
                  <div className="px-4 py-2 border-t border-gray-100 bg-gray-50 text-xs text-gray-500 flex justify-between items-center">
                    <div>
                      Last updated: {lastFetched.toLocaleTimeString()}
                    </div>
                    <div>
                      <a 
                        href={`${API_BASE_URL}/${entity.type}/${entity.pubkey}`} 
                        target="_blank" 
                        rel="noopener noreferrer"
                        className="text-indigo-600 hover:text-indigo-800"
                      >
                        View API →
                      </a>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
} 