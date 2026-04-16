#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Ledger, Env, String};
    use crate::{RemittanceContractClient, TransferStatus};

    fn setup() -> (Env, RemittanceContractClient<'static>) {
        let env = Env::default();
        env.ledger().set_timestamp(1_700_000_000);
        let contract_id = env.register_contract(None, crate::RemittanceContract);
        let client = RemittanceContractClient::new(&env, &contract_id);
        (env, client)
    }

    // -------------------------------------------------------
    // TEST 1: TKI berhasil kirim uang ke keluarga
    // -------------------------------------------------------
    #[test]
    fn test_send_money_success() {
        let (env, client) = setup();

        let transfer_id = client.send_money(
            &String::from_str(&env, "Siti Aminah"),
            &String::from_str(&env, "3578010101950001"),
            &String::from_str(&env, "Malaysia"),
            &String::from_str(&env, "Bapak Suwarno"),
            &String::from_str(&env, "3578010101600001"),
            &String::from_str(&env, "08123456789"),
            &20000u64,     // $200.00 USD (dalam sen)
            &3_100_000u64, // Rp 3.100.000
            &String::from_str(&env, "Buat bayar sekolah adik ya Pak"),
        );

        // Transfer ID harus lebih dari 0
        assert!(transfer_id > 0);

        // Cek transfer bisa ditemukan
        let result = client.get_transfer_by_id(&transfer_id);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().sender_name, String::from_str(&env, "Siti Aminah"));
        assert_eq!(result.get(0).unwrap().amount_usd, 20000u64);
    }

    // -------------------------------------------------------
    // TEST 2: Keluarga berhasil klaim kiriman
    // -------------------------------------------------------
    #[test]
    fn test_claim_money_success() {
        let (env, client) = setup();

        let transfer_id = client.send_money(
            &String::from_str(&env, "Dewi Lestari"),
            &String::from_str(&env, "3201010202930002"),
            &String::from_str(&env, "Hongkong"),
            &String::from_str(&env, "Ibu Parti"),
            &String::from_str(&env, "3201010202650002"),
            &String::from_str(&env, "08567890123"),
            &15000u64,
            &2_300_000u64,
            &String::from_str(&env, "Buat makan sehari-hari Bu"),
        );

        let result = client.claim_money(
            &transfer_id,
            &String::from_str(&env, "3201010202650002"),
        );

        assert_eq!(result, String::from_str(&env, "Transfer claimed successfully"));

        // Verifikasi status berubah jadi Claimed
        let transfer = client.get_transfer_by_id(&transfer_id);
        assert_eq!(transfer.get(0).unwrap().status, TransferStatus::Claimed);
    }

    // -------------------------------------------------------
    // TEST 3: Klaim gagal jika ID penerima salah
    // -------------------------------------------------------
    #[test]
    fn test_claim_with_wrong_recipient_id() {
        let (env, client) = setup();

        let transfer_id = client.send_money(
            &String::from_str(&env, "Rina Kartika"),
            &String::from_str(&env, "3374010303940003"),
            &String::from_str(&env, "Taiwan"),
            &String::from_str(&env, "Pak Warno"),
            &String::from_str(&env, "3374010303660003"),
            &String::from_str(&env, "08234567890"),
            &10000u64,
            &1_550_000u64,
            &String::from_str(&env, "Titip buat bayar listrik"),
        );

        // Coba klaim dengan NIK yang salah
        let result = client.claim_money(
            &transfer_id,
            &String::from_str(&env, "9999999999999999"),
        );

        assert_eq!(result, String::from_str(&env, "ERROR: Recipient ID does not match"));
    }

    // -------------------------------------------------------
    // TEST 4: Tidak bisa klaim dua kali
    // -------------------------------------------------------
    #[test]
    fn test_cannot_claim_twice() {
        let (env, client) = setup();

        let transfer_id = client.send_money(
            &String::from_str(&env, "Nurul Fadilah"),
            &String::from_str(&env, "3273010404920004"),
            &String::from_str(&env, "Saudi Arabia"),
            &String::from_str(&env, "Ibu Romlah"),
            &String::from_str(&env, "3273010404640004"),
            &String::from_str(&env, "08345678901"),
            &25000u64,
            &3_875_000u64,
            &String::from_str(&env, "Lebaran sebentar lagi Bu"),
        );

        // Klaim pertama berhasil
        client.claim_money(
            &transfer_id,
            &String::from_str(&env, "3273010404640004"),
        );

        // Klaim kedua harus gagal
        let result = client.claim_money(
            &transfer_id,
            &String::from_str(&env, "3273010404640004"),
        );

        assert_eq!(result, String::from_str(&env, "ERROR: Transfer has already been claimed"));
    }

    // -------------------------------------------------------
    // TEST 5: TKI bisa batalkan transfer yang masih pending
    // -------------------------------------------------------
    #[test]
    fn test_cancel_transfer() {
        let (env, client) = setup();

        let transfer_id = client.send_money(
            &String::from_str(&env, "Yuli Astuti"),
            &String::from_str(&env, "3505010505910005"),
            &String::from_str(&env, "Singapura"),
            &String::from_str(&env, "Suami"),
            &String::from_str(&env, "3505010505890005"),
            &String::from_str(&env, "08456789012"),
            &5000u64,
            &775_000u64,
            &String::from_str(&env, "Salah kirim, tolong cancel"),
        );

        let result = client.cancel_transfer(
            &transfer_id,
            &String::from_str(&env, "3505010505910005"),
        );

        assert_eq!(result, String::from_str(&env, "Transfer cancelled successfully"));

        // Setelah dibatalkan, tidak bisa diklaim
        let claim = client.claim_money(
            &transfer_id,
            &String::from_str(&env, "3505010505890005"),
        );
        assert_eq!(claim, String::from_str(&env, "ERROR: Transfer has been cancelled"));
    }

    // -------------------------------------------------------
    // TEST 6: Filter riwayat per pengirim
    // -------------------------------------------------------
    #[test]
    fn test_get_transfers_by_sender() {
        let (env, client) = setup();

        // TKI yang sama kirim 2x
        client.send_money(
            &String::from_str(&env, "Siti Aminah"),
            &String::from_str(&env, "3578010101950001"),
            &String::from_str(&env, "Malaysia"),
            &String::from_str(&env, "Bapak"),
            &String::from_str(&env, "3578010101600001"),
            &String::from_str(&env, "08111111111"),
            &10000u64,
            &1_550_000u64,
            &String::from_str(&env, "Kiriman bulan Januari"),
        );

        client.send_money(
            &String::from_str(&env, "Siti Aminah"),
            &String::from_str(&env, "3578010101950001"),
            &String::from_str(&env, "Malaysia"),
            &String::from_str(&env, "Bapak"),
            &String::from_str(&env, "3578010101600001"),
            &String::from_str(&env, "08111111111"),
            &10000u64,
            &1_550_000u64,
            &String::from_str(&env, "Kiriman bulan Februari"),
        );

        let transfers = client.get_transfers_by_sender(
            &String::from_str(&env, "3578010101950001"),
        );

        assert_eq!(transfers.len(), 2);
    }
}
