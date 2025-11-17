use std::collections::HashSet;

// Section 6: Cross-Border Risk Model
enum RiskZone {
    Low,
    Medium,
    High,
}

pub fn get_country_risk(country_code: &str) -> (RiskZone, i32) {
    let code = country_code.to_uppercase();
    
    // LOW RISK (Section 6)
    let low_risk: HashSet<&str> = [
        "UK", "US", "EU", "CA", "AU", "KE", "GH", "NG",
    ].into_iter().collect();
    
    // MEDIUM RISK (Section 6)
    let medium_risk: HashSet<&str> = [
        "AE", "CN", "IN", "TR",
    ].into_iter().collect();

    if low_risk.contains(code.as_str()) {
        (RiskZone::Low, -15) // Section 14
    } else if medium_risk.contains(code.as_str()) {
        (RiskZone::Medium, 10) // Section 14
    } else {
        // Default to HIGH RISK for all others
        (RiskZone::High, 40) // Section 14
    }
}

// Section 5: Spending Behavior
pub enum SpendingCategory {
    Business,
    Individual,
    PeraToPera,
}

pub fn classify_spending(channel: &str, beneficiary_name: &str) -> (SpendingCategory, i32) {
    let name = beneficiary_name.to_lowercase();
    if name.contains("ltd") || name.contains("inc") || name.contains("limited") {
        (SpendingCategory::Business, -10) // Section 14
    } else if channel == "pera" {
        (SpendingCategory::PeraToPera, 0) // P2P score is handled separately
    } else {
        (SpendingCategory::Individual, 10) // Section 14
    }
}