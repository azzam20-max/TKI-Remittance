#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    symbol_short, Env, String, Symbol, Vec,
};

// =============================================
// ENUM STATUS PENGIRIMAN
// =============================================

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum TransferStatus {
    Pending,    // Uang sudah dikirim, belum diklaim
    Claimed,    // Uang sudah diklaim keluarga
    Cancelled,  // Dibatalkan oleh pengirim
}

// =============================================
// STRUKTUR DATA
// =============================================

/// Data setiap transaksi pengiriman uang
#[contracttype]
#[derive(Clone, Debug)]
pub struct RemittanceRecord {
    pub transfer_id: u64,           // ID unik transaksi
    pub sender_name: String,        // Nama TKI pengirim
    pub sender_id: String,          // NIK / ID TKI
    pub sender_country: String,     // Negara tempat TKI bekerja
    pub recipient_name: String,     // Nama penerima (keluarga)
    pub recipient_id: String,       // NIK penerima
    pub recipient_phone: String,    // No. HP penerima
    pub amount_usd: u64,            // Jumlah uang (dalam USD sen, misal 10000 = $100.00)
    pub amount_idr: u64,            // Estimasi IDR (dalam rupiah)
    pub message: String,            // Pesan dari TKI ke keluarga
    pub status: TransferStatus,     // Status pengiriman
    pub sent_at: u64,               // Timestamp pengiriman
    pub claimed_at: u64,            // Timestamp klaim (0 jika belum)
}

// =============================================
// STORAGE KEYS
// =============================================
const REMIT_DATA: Symbol = symbol_short!("REMITDATA");

// =============================================
// CONTRACT
// =============================================
#[contract]
pub struct RemittanceContract;

#[contractimpl]
impl RemittanceContract {

    // --------------------------------------------------
    // 1. Kirim uang (oleh TKI dari luar negeri)
    // --------------------------------------------------
    pub fn send_money(
        env: Env,
        sender_name: String,
        sender_id: String,
        sender_country: String,
        recipient_name: String,
        recipient_id: String,
        recipient_phone: String,
        amount_usd: u64,
        amount_idr: u64,
        message: String,
    ) -> u64 {
        // Validasi: jumlah harus lebih dari 0
        if amount_usd == 0 {
            return 0; // Return 0 sebagai tanda error
        }

        let mut records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        let transfer_id = env.prng().gen::<u64>();

        let record = RemittanceRecord {
            transfer_id,
            sender_name,
            sender_id,
            sender_country,
            recipient_name,
            recipient_id,
            recipient_phone,
            amount_usd,
            amount_idr,
            message,
            status: TransferStatus::Pending,
            sent_at: env.ledger().timestamp(),
            claimed_at: 0u64,
        };

        records.push_back(record);
        env.storage().instance().set(&REMIT_DATA, &records);

        // Return transfer_id supaya pengirim bisa simpan sebagai bukti
        transfer_id
    }

    // --------------------------------------------------
    // 2. Klaim uang (oleh keluarga penerima)
    // --------------------------------------------------
    pub fn claim_money(env: Env, transfer_id: u64, recipient_id: String) -> String {
        let mut records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        for i in 0..records.len() {
            let record = records.get(i).unwrap();

            if record.transfer_id == transfer_id {
                // Verifikasi ID penerima cocok
                if record.recipient_id != recipient_id {
                    return String::from_str(&env, "ERROR: Recipient ID does not match");
                }

                // Cek status
                if record.status == TransferStatus::Claimed {
                    return String::from_str(&env, "ERROR: Transfer has already been claimed");
                }

                if record.status == TransferStatus::Cancelled {
                    return String::from_str(&env, "ERROR: Transfer has been cancelled");
                }

                // Update status menjadi Claimed
                let updated = RemittanceRecord {
                    transfer_id: record.transfer_id,
                    sender_name: record.sender_name,
                    sender_id: record.sender_id,
                    sender_country: record.sender_country,
                    recipient_name: record.recipient_name,
                    recipient_id: record.recipient_id,
                    recipient_phone: record.recipient_phone,
                    amount_usd: record.amount_usd,
                    amount_idr: record.amount_idr,
                    message: record.message,
                    status: TransferStatus::Claimed,
                    sent_at: record.sent_at,
                    claimed_at: env.ledger().timestamp(),
                };

                records.set(i, updated);
                env.storage().instance().set(&REMIT_DATA, &records);

                return String::from_str(&env, "Transfer claimed successfully");
            }
        }

        String::from_str(&env, "ERROR: Transfer ID not found")
    }

    // --------------------------------------------------
    // 3. Batalkan pengiriman (hanya jika masih Pending)
    // --------------------------------------------------
    pub fn cancel_transfer(env: Env, transfer_id: u64, sender_id: String) -> String {
        let mut records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        for i in 0..records.len() {
            let record = records.get(i).unwrap();

            if record.transfer_id == transfer_id {
                // Verifikasi ID pengirim cocok
                if record.sender_id != sender_id {
                    return String::from_str(&env, "ERROR: Sender ID does not match");
                }

                if record.status != TransferStatus::Pending {
                    return String::from_str(&env, "ERROR: Only pending transfers can be cancelled");
                }

                let updated = RemittanceRecord {
                    transfer_id: record.transfer_id,
                    sender_name: record.sender_name,
                    sender_id: record.sender_id,
                    sender_country: record.sender_country,
                    recipient_name: record.recipient_name,
                    recipient_id: record.recipient_id,
                    recipient_phone: record.recipient_phone,
                    amount_usd: record.amount_usd,
                    amount_idr: record.amount_idr,
                    message: record.message,
                    status: TransferStatus::Cancelled,
                    sent_at: record.sent_at,
                    claimed_at: 0u64,
                };

                records.set(i, updated);
                env.storage().instance().set(&REMIT_DATA, &records);

                return String::from_str(&env, "Transfer cancelled successfully");
            }
        }

        String::from_str(&env, "ERROR: Transfer ID not found")
    }

    // --------------------------------------------------
    // 4. Lihat semua riwayat pengiriman milik satu TKI
    // --------------------------------------------------
    pub fn get_transfers_by_sender(env: Env, sender_id: String) -> Vec<RemittanceRecord> {
        let records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<RemittanceRecord> = Vec::new(&env);

        for i in 0..records.len() {
            let record = records.get(i).unwrap();
            if record.sender_id == sender_id {
                result.push_back(record);
            }
        }

        result
    }

    // --------------------------------------------------
    // 5. Lihat semua kiriman yang masuk ke penerima
    // --------------------------------------------------
    pub fn get_transfers_by_recipient(env: Env, recipient_id: String) -> Vec<RemittanceRecord> {
        let records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<RemittanceRecord> = Vec::new(&env);

        for i in 0..records.len() {
            let record = records.get(i).unwrap();
            if record.recipient_id == recipient_id {
                result.push_back(record);
            }
        }

        result
    }

    // --------------------------------------------------
    // 6. Cek status 1 transfer berdasarkan ID
    // --------------------------------------------------
    pub fn get_transfer_by_id(env: Env, transfer_id: u64) -> Vec<RemittanceRecord> {
        let records: Vec<RemittanceRecord> = env
            .storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env));

        let mut result: Vec<RemittanceRecord> = Vec::new(&env);

        for i in 0..records.len() {
            let record = records.get(i).unwrap();
            if record.transfer_id == transfer_id {
                result.push_back(record);
                return result;
            }
        }

        result
    }

    // --------------------------------------------------
    // 7. Lihat semua transaksi (untuk monitoring)
    // --------------------------------------------------
    pub fn get_all_transfers(env: Env) -> Vec<RemittanceRecord> {
        env.storage().instance()
            .get(&REMIT_DATA)
            .unwrap_or(Vec::new(&env))
    }
}

mod test;
