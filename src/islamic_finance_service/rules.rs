use std::collections::HashSet;

/// Checks if a Merchant Category Code (MCC) is blocked.
/// This is the core of the "Islamic note" policy.
///
/// In a real app, this list would be in the database.
pub fn is_mcc_blocked(mcc: &str) -> bool {
    // A mock set of non-Halal MCCs
    let blocked_mccs: HashSet<&str> = [
        "5921", // Alcohol, Packaged Beer
        "7995", // Gambling
        "7273", // Dating Services
        "5993", // Tobacco
                // ... etc.
    ]
    .into_iter()
    .collect();

    blocked_mccs.contains(mcc)
}

// --- ADDED THIS FUNCTION (Section 12) ---
/// Checks if a payment beneficiary is blocked by Islamic ethics
pub fn is_beneficiary_blocked(beneficiary_name: &str, channel: &str) -> bool {
    let name = beneficiary_name.to_lowercase();

    if channel == "bank" {
        // Block interest-based transactions
        if name.contains("loan") || name.contains("interest") || name.contains("finance co") {
            return true;
        }
    }

    // Block gambling, alcohol, etc.
    if name.contains("casino") || name.contains("betting") || name.contains("liquor") {
        return true;
    }

    false
}
// -------------------------------------
