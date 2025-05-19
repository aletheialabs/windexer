'use client';

import React, { useState, useEffect } from 'react';
import { 
  ChevronRight,
  ChevronDown,
  LayoutDashboard,
  Layers,
  Hexagon,
  Hash, 
  Clock, 
  Database, 
  AlertCircle, 
  Wallet, 
  FileText, 
  Radio, 
  CheckCircle, 
  XCircle,
  Droplets
} from 'lucide-react';
import MultiIndex from './components/MultiIndex';
import BlockExplorer from './components/BlockExplorer';
import LoadingSpinner from './components/LoadingSpinner';
import TransactionViewer from './components/TransactionViewer';
import RaydiumTracker from './components/RaydiumTracker';
import {
  StatusResponse,
  HealthResponse,
  MetricsResponse,
  SidebarProps,
  DashboardCardProps,
  EndpointSectionProps,
  EndpointInfo
} from './types';

// Base API URL
const API_BASE_URL = "http://localhost:3000/api";

const App = () => {
  // State for active tab
  const [activeTab, setActiveTab] = useState('dashboard');
  
  return (
    <div className="flex flex-col min-h-screen bg-gray-50 dark:bg-gray-900">
      <Header />
      <div className="flex flex-1">
        <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} />
        <main className="flex-1 p-6 overflow-x-auto">
          {activeTab === 'dashboard' && <Dashboard />}
          {activeTab === 'blocks' && <BlockExplorer />}
          {activeTab === 'transactions' && <TransactionViewer />}
          {activeTab === 'accounts' && <AccountInspector />}
          {activeTab === 'multi-index' && <MultiIndex />}
          {activeTab === 'websocket' && <WebSocketDemo />}
          {activeTab === 'raydium' && <RaydiumTracker />}
        </main>
      </div>
      <Footer />
    </div>
  );
};

const Header = () => {
  return (
    <header className="sticky top-0 z-10 bg-gradient-to-r from-indigo-800 to-purple-700 text-white p-4 shadow-md">
      <div className="container mx-auto flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Hexagon className="h-6 w-6 text-indigo-200" />
          <h1 className="text-xl font-bold">Wind Network Explorer</h1>
        </div>
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-1">
            <div className="h-2 w-2 rounded-full bg-green-400"></div>
            <span className="text-sm">Decentralized Solana Indexer</span>
          </div>
        </div>
      </div>
    </header>
  );
};

const Sidebar: React.FC<SidebarProps> = ({ activeTab, setActiveTab }) => {
  const tabs = [
    { id: 'dashboard', name: 'Dashboard', icon: <LayoutDashboard className="h-5 w-5" /> },
    { id: 'blocks', name: 'Block Explorer', icon: <Hash className="h-5 w-5" /> },
    { id: 'transactions', name: 'Transactions', icon: <FileText className="h-5 w-5" /> },
    { id: 'accounts', name: 'Accounts', icon: <Wallet className="h-5 w-5" /> },
    { id: 'multi-index', name: 'Multi-Index', icon: <Layers className="h-5 w-5" /> },
    { id: 'websocket', name: 'Live Updates', icon: <Radio className="h-5 w-5" /> },
    { id: 'raydium', name: 'Raydium Tracker', icon: <Droplets className="h-5 w-5" /> },
  ];

  return (
    <aside className="w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 shadow-sm">
      <nav className="p-4">
        <ul className="space-y-2">
          {tabs.map((tab) => (
            <li key={tab.id}>
              <button
                onClick={() => setActiveTab(tab.id)}
                className={`flex items-center space-x-3 w-full p-3 rounded-md transition-colors ${
                  activeTab === tab.id
                    ? 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200'
                    : 'hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-gray-200'
                }`}
              >
                {tab.icon}
                <span>{tab.name}</span>
              </button>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
};

const Footer = () => {
  return (
    <footer className="bg-gray-800 text-gray-300 p-4 text-center text-sm">
      <p>Wind Network - Decentralized Indexing Solution for Solana</p>
    </footer>
  );
};

const Dashboard = () => {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      try {
        const [healthRes, statusRes, metricsRes] = await Promise.all([
          fetch(`${API_BASE_URL}/health`),
          fetch(`${API_BASE_URL}/status`),
          fetch(`${API_BASE_URL}/metrics`)
        ]);

        const healthData = await healthRes.json();
        const statusData = await statusRes.json();
        const metricsData = await metricsRes.json();

        setHealth(healthData);
        setStatus(statusData);
        setMetrics(metricsData);
      } catch (error) {
        console.error('Error fetching dashboard data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 10000); // Refresh every 10 seconds
    return () => clearInterval(interval);
  }, []);

  if (loading && !health) {
    return <LoadingSpinner message="Loading system status..." />;
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-gray-800 dark:text-white">System Dashboard</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Health Card */}
        <DashboardCard 
          title="System Health" 
          icon={<AlertCircle className="h-6 w-6 text-indigo-600 dark:text-indigo-400" />}
        >
          <div className="flex items-center space-x-2">
            {health?.status === "healthy" ? (
              <>
                <CheckCircle className="h-5 w-5 text-green-500" />
                <span className="text-green-600 dark:text-green-400 font-medium">System Healthy</span>
              </>
            ) : (
              <>
                <XCircle className="h-5 w-5 text-red-500" />
                <span className="text-red-600 dark:text-red-400 font-medium">System Degraded</span>
              </>
            )}
          </div>
          <div className="mt-2 text-sm text-gray-600 dark:text-gray-400">
            Last checked: {new Date().toLocaleTimeString()}
          </div>
        </DashboardCard>

        {/* Status Card */}
        <DashboardCard 
          title="Network Status" 
          icon={<Database className="h-6 w-6 text-indigo-600 dark:text-indigo-400" />}
        >
          {status ? (
            <div className="space-y-2">
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Version:</span>
                <span className="font-medium dark:text-white">{status.version || 'N/A'}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Nodes:</span>
                <span className="font-medium dark:text-white">{status.nodes || 0}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Latest Slot:</span>
                <span className="font-medium dark:text-white">{status.latest_slot || 'N/A'}</span>
              </div>
            </div>
          ) : (
            <div className="text-sm text-gray-600 dark:text-gray-400">Status information unavailable</div>
          )}
        </DashboardCard>

        {/* Metrics Card */}
        <DashboardCard 
          title="Performance Metrics" 
          icon={<Clock className="h-6 w-6 text-indigo-600 dark:text-indigo-400" />}
        >
          {metrics ? (
            <div className="space-y-2">
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Transactions Indexed:</span>
                <span className="font-medium dark:text-white">{metrics.transactions_indexed?.toLocaleString() || 0}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Blocks Processed:</span>
                <span className="font-medium dark:text-white">{metrics.blocks_processed?.toLocaleString() || 0}</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-sm text-gray-600 dark:text-gray-400">Query Response Time:</span>
                <span className="font-medium dark:text-white">{metrics.avg_query_time || '50'} ms</span>
              </div>
            </div>
          ) : (
            <div className="text-sm text-gray-600 dark:text-gray-400">Metrics information unavailable</div>
          )}
        </DashboardCard>
      </div>

      {/* API Endpoints Section */}
      <div className="mt-8 bg-white dark:bg-gray-800 p-6 rounded-lg shadow">
        <h3 className="text-lg font-semibold text-gray-800 dark:text-white mb-4">Available API Endpoints</h3>
        
        <div className="space-y-4">
          <EndpointSection title="Status & Health" endpoints={[
            { url: "/api/status", description: "Network status information" },
            { url: "/api/health", description: "System health check" },
            { url: "/api/metrics", description: "Performance metrics" }
          ]} />

          <EndpointSection title="Block Data" endpoints={[
            { url: "/api/blocks/latest", description: "Latest processed blocks" },
            { url: "/api/blocks/{slot}", description: "Block data by slot number" }
          ]} />

          <EndpointSection title="Transaction Data" endpoints={[
            { url: "/api/transaction/{signature}", description: "Transaction by signature" },
            { url: "/api/transactions/recent", description: "Recently processed transactions" }
          ]} />

          <EndpointSection title="Account Data" endpoints={[
            { url: "/api/account/{pubkey}", description: "Account data by public key" },
            { url: "/api/account/{pubkey}/balance", description: "Account balance" },
            { url: "/api/account/{pubkey}/tokens", description: "Account token balances" },
            { url: "/api/accounts/program/{program_id}", description: "Accounts by program ID" }
          ]} />
        </div>
      </div>
    </div>
  );
};

const EndpointSection: React.FC<EndpointSectionProps> = ({ title, endpoints }) => {
  const [isOpen, setIsOpen] = useState(false);
  
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden">
      <button 
        className="w-full flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700 text-left font-medium dark:text-white"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span>{title}</span>
        {isOpen ? <ChevronDown className="h-5 w-5" /> : <ChevronRight className="h-5 w-5" />}
      </button>
      
      {isOpen && (
        <div className="p-3 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          <ul className="space-y-2">
            {endpoints.map((endpoint, index) => (
              <li key={index} className="flex">
                <code className="bg-gray-100 dark:bg-gray-900 text-pink-600 dark:text-pink-400 p-1 rounded font-mono text-sm mr-3 flex-shrink-0">
                  {endpoint.url}
                </code>
                <span className="text-sm text-gray-600 dark:text-gray-400">{endpoint.description}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};

const DashboardCard: React.FC<DashboardCardProps> = ({ title, icon, children }) => {
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <div className="flex items-center space-x-2 mb-4">
        {icon}
        <h3 className="font-semibold text-gray-800 dark:text-white">{title}</h3>
      </div>
      <div>{children}</div>
    </div>
  );
};

// Placeholder for components we haven't moved to separate files yet
const AccountInspector = () => {
  return <div>Account Inspector Component (To be implemented)</div>;
};

const WebSocketDemo = () => {
  return <div>WebSocket Demo Component (To be implemented)</div>;
};

export default App;