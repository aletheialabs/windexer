// crates/windexer-jito-staking/src/slashing/monitor.rs

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;
use crate::slashing::ViolationType;

pub struct SlashingMonitor {
    slash_threshold: f64,
    min_uptime: f64,
    violation_history: HashMap<Pubkey, Vec<ViolationRecord>>,
}

#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub timestamp: i64,
    pub violation_type: ViolationType,
    pub severity: f64,
}

impl SlashingMonitor {
    pub fn new(slash_threshold: f64, min_uptime: f64) -> Self {
        Self {
            slash_threshold,
            min_uptime,
            violation_history: HashMap::new(),
        }
    }

    pub async fn should_slash(&mut self, operator: &Pubkey, violation: &ViolationType) -> Result<bool> {
        let severity = self.calculate_violation_severity(violation);
        let records = self.violation_history
            .entry(*operator)
            .or_default();
            
        let violation_record = ViolationRecord {
            timestamp: chrono::Utc::now().timestamp(),
            violation_type: violation.clone(),
            severity,
        };
        
        records.push(violation_record);
        self.check_slash_threshold(operator)
    }

    fn calculate_violation_severity(&self, violation: &ViolationType) -> f64 {
        match violation {
            ViolationType::Downtime => 0.5,
            ViolationType::InvalidConsensus => 0.8,
            ViolationType::MaliciousBehavior => 1.0,
        }
    }

    fn check_slash_threshold(&self, operator: &Pubkey) -> Result<bool> {
        let records = self.violation_history.get(operator)
            .ok_or_else(|| anyhow::anyhow!("No violation history found"))?;
            
        let total_severity: f64 = records.iter()
            .map(|r| r.severity)
            .sum();
            
        Ok(total_severity >= self.slash_threshold)
    }
}