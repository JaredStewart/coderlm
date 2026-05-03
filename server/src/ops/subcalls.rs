use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcallResult {
    pub chunk_id: String,
    pub query: String,
    pub findings: Vec<String>,
    pub suggested_queries: Vec<String>,
    pub answer_if_complete: Option<String>,
    pub depth: u32,
}

pub fn store_result(results: &Mutex<Vec<SubcallResult>>, result: SubcallResult) {
    results.lock().push(result);
}

pub fn get_results(results: &Mutex<Vec<SubcallResult>>) -> Vec<SubcallResult> {
    results.lock().clone()
}

pub fn clear_results(results: &Mutex<Vec<SubcallResult>>) {
    results.lock().clear();
}
