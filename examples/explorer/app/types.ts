// API Response types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string | ApiError;
}

export interface ApiError {
  message: string;
  code?: number;
}

// Dashboard types
export interface StatusResponse {
  name: string;
  version: string;
  uptime: number;
  timestamp: string;
  nodes?: number;
  latest_slot?: number;
}

export interface HealthResponse {
  status: string;
  uptime: number;
  timestamp: string;
  checks: Record<string, boolean>;
}

export interface MetricsResponse {
  memory_usage?: number;
  cpu_usage?: number;
  active_connections?: number;
  transactions_indexed?: number;
  blocks_processed?: number;
  avg_query_time?: number;
}

// Block related types
export interface BlockData {
  slot: number;
  hash: string;
  parent_hash: string;
  block_time: number;
  block_height: number;
  transactions: number;
  parent_slot?: number;
  blockhash?: string;
  previous_blockhash?: string;
  transaction_count?: number;
  leader?: string;
}

// Transaction related types
export interface TransactionData {
  signature: string;
  slot: number;
  block_time?: number;
  fee: number;
  status: string;
  success?: boolean;
  err?: any;
  recent_blockhash?: string;
  program_ids?: string[];
  accounts?: string[];
  logs?: string[];
  instructions?: InstructionData[];
}

export interface InstructionData {
  program_id: string;
  accounts: string[];
  data: string;
}

// Account related types
export interface AccountData {
  pubkey: string;
  lamports: number;
  owner: string;
  executable: boolean;
  rent_epoch: number;
  data: any[];
  data_base64?: string;
  slot?: number;
  updated_at?: number;
  data_size?: number;
}

export interface TokenBalance {
  mint: string;
  owner: string;
  amount: number;
  decimals: number;
  symbol?: string;
  name?: string;
  logo?: string;
  balance?: number;
}

// UI Component props
export interface EndpointInfo {
  url: string;
  description: string;
}

export interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

export interface DashboardCardProps {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}

export interface EndpointSectionProps {
  title: string;
  endpoints: EndpointInfo[];
}

// WebSocket related types
export interface WebSocketMessage {
  id: number;
  timestamp: Date;
  source: string;
  content: any;
} 