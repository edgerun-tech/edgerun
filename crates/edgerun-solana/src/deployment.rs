//! DeploymentContract Client
//!
//! Minimal JSON-RPC client using edgerun-http and bincode.

use crate::prelude::*;
use crate::signers::Signer;
use crate::solana_types::{AccountMeta, Instruction, Pubkey};
use edgerun_http::{HttpClient, HttpVersion};
use edgerun_json::{JsonValue, json};
use std::sync::Arc;

use crate::error::SolanaError;
use crate::try_deployment_program_id;
use crate::types::{Deployment, DeploymentStatus};

const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);
const DEPLOYMENT_ACCOUNT_SIZE: u64 = 448;

fn make_instruction(
    program_id: Pubkey,
    variant: u8,
    data: &[u8],
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut bytes = vec![variant];
    bytes.extend_from_slice(data);
    Instruction {
        program_id,
        accounts,
        data: bytes,
    }
}

pub struct DeploymentClient {
    http: HttpClient,
    program_id: Pubkey,
    rpc_url: String,
    runtime: Arc<HttpRuntime>,
}

#[derive(Debug, Clone)]
pub struct DeploymentAccount {
    pub pubkey: Pubkey,
    pub deployment: Deployment,
}

impl DeploymentClient {
    pub fn new(rpc_url: &str) -> Result<Self, SolanaError> {
        Self::new_with_program_id(rpc_url, try_deployment_program_id()?)
    }

    pub fn new_with_program_id(rpc_url: &str, program_id: Pubkey) -> Result<Self, SolanaError> {
        let rt = HttpRuntime::new();
        Ok(Self {
            http: HttpClient::new().version(HttpVersion::Http1).no_redirects(),
            program_id,
            rpc_url: rpc_url.to_string(),
            runtime: Arc::new(rt),
        })
    }

    fn rpc_call(&self, payload: JsonValue) -> Result<JsonValue, SolanaError> {
        let json_str = payload
            .to_json_string()
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let rpc_url = self.rpc_url.clone();
        let http = self.http.clone();

        let resp = self
            .runtime
            .block_on(async move { http.post_json(&rpc_url, &json_str).await })
            .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let body =
            String::from_utf8(resp.body().to_vec()).map_err(|e| SolanaError::Rpc(e.to_string()))?;
        let parsed =
            edgerun_json::parse_json(&body).map_err(|e| SolanaError::Rpc(e.to_string()))?;
        if parsed.get("error").is_some() {
            return Err(SolanaError::Rpc(
                parsed["error"].to_json_string().unwrap_or_default(),
            ));
        }
        Ok(parsed)
    }

    pub fn get_deployment(&self, deployment_pubkey: &Pubkey) -> Result<Deployment, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [deployment_pubkey.to_string(), { "encoding": "base64", "commitment": "confirmed" }]
        });
        let resp = self.rpc_call(payload)?;
        let bytes = decode_rpc_account_data(&resp)?;
        Deployment::try_from_slice(&bytes).map_err(|e| SolanaError::Serialization(e.to_string()))
    }

    pub fn list_deployments(&self) -> Result<Vec<DeploymentAccount>, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "getProgramAccounts",
            "params": [
                self.program_id.to_string(),
                { "encoding": "base64", "commitment": "confirmed" }
            ]
        });
        let resp = self.rpc_call(payload)?;
        let accounts = resp["result"]
            .as_array()
            .ok_or_else(|| SolanaError::Rpc("no deployment accounts".to_string()))?;
        let mut deployments = Vec::new();
        for account in accounts {
            let pubkey = account["pubkey"]
                .as_str()
                .ok_or_else(|| SolanaError::Rpc("program account missing pubkey".to_string()))?
                .parse::<Pubkey>()
                .map_err(|err| SolanaError::Rpc(format!("invalid deployment pubkey: {err}")))?;
            let bytes = decode_rpc_account_data_value(&account["account"]["data"])?;
            match Deployment::try_from_slice(&bytes) {
                Ok(deployment) => deployments.push(DeploymentAccount { pubkey, deployment }),
                Err(_) => continue,
            }
        }
        Ok(deployments)
    }

    pub fn initialize_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        name: [u8; 64],
        provider: [u8; 32],
        container_count: u32,
        total_cpu_cores: u32,
        total_memory_bytes: u64,
        total_storage_bytes: u64,
        total_network_mbps: u32,
        deposit: u64,
        burn_rate: u64,
        auto_stop_on_price_increase: bool,
        governance_authority: [u8; 32],
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&name);
        encoded.extend_from_slice(&provider);
        encoded.extend_from_slice(&container_count.to_le_bytes());
        encoded.extend_from_slice(&total_cpu_cores.to_le_bytes());
        encoded.extend_from_slice(&total_memory_bytes.to_le_bytes());
        encoded.extend_from_slice(&total_storage_bytes.to_le_bytes());
        encoded.extend_from_slice(&total_network_mbps.to_le_bytes());
        encoded.extend_from_slice(&deposit.to_le_bytes());
        encoded.extend_from_slice(&burn_rate.to_le_bytes());
        encoded.push(u8::from(auto_stop_on_price_increase));
        encoded.extend_from_slice(&governance_authority);
        make_instruction(
            self.program_id,
            0,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID),
            ],
        )
    }

    pub fn deployment_address_with_seed(
        &self,
        owner_pubkey: &Pubkey,
        seed: &str,
    ) -> Result<Pubkey, SolanaError> {
        create_address_with_seed(owner_pubkey, seed, &self.program_id)
    }

    pub async fn create_deployment_signed<S: Signer>(
        &self,
        owner_pubkey: &Pubkey,
        seed: &str,
        signer: &S,
        name: [u8; 64],
        provider: [u8; 32],
        container_count: u32,
        total_cpu_cores: u32,
        total_memory_bytes: u64,
        total_storage_bytes: u64,
        total_network_mbps: u32,
        deposit: u64,
        burn_rate: u64,
        auto_stop_on_price_increase: bool,
        governance_authority: [u8; 32],
    ) -> Result<(Pubkey, String), SolanaError> {
        let deployment_pubkey = self.deployment_address_with_seed(owner_pubkey, seed)?;
        let rent_lamports = self.minimum_balance_for_rent_exemption(DEPLOYMENT_ACCOUNT_SIZE)?;
        let create_ix = create_account_with_seed_instruction(
            owner_pubkey,
            &deployment_pubkey,
            owner_pubkey,
            seed,
            rent_lamports,
            DEPLOYMENT_ACCOUNT_SIZE,
            &self.program_id,
        )?;
        let init_ix = self.initialize_instruction(
            &deployment_pubkey,
            owner_pubkey,
            name,
            provider,
            container_count,
            total_cpu_cores,
            total_memory_bytes,
            total_storage_bytes,
            total_network_mbps,
            deposit,
            burn_rate,
            auto_stop_on_price_increase,
            governance_authority,
        );
        let tx = self
            .send_instructions_signed(&[create_ix, init_ix], owner_pubkey, signer)
            .await?;
        Ok((deployment_pubkey, tx))
    }

    pub fn report_metrics_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
        provider_authority_pubkey: &Pubkey,
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        storage_bytes_used: u64,
        network_bytes_sent: u64,
        container_count: u32,
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&cpu_cores_used.to_le_bytes());
        encoded.extend_from_slice(&memory_bytes_used.to_le_bytes());
        encoded.extend_from_slice(&storage_bytes_used.to_le_bytes());
        encoded.extend_from_slice(&network_bytes_sent.to_le_bytes());
        encoded.extend_from_slice(&container_count.to_le_bytes());
        make_instruction(
            self.program_id,
            8,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*provider_pubkey, false),
                AccountMeta::new(*provider_authority_pubkey, true),
            ],
        )
    }

    pub fn start_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            1,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

    pub fn assign_provider_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        scheduler_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            10,
            provider_pubkey.as_bytes(),
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*scheduler_pubkey, true),
                AccountMeta::new(*provider_pubkey, false),
            ],
        )
    }

    pub async fn assign_provider_signed<S: Signer>(
        &self,
        deployment_pubkey: &Pubkey,
        scheduler_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        let ix =
            self.assign_provider_instruction(deployment_pubkey, scheduler_pubkey, provider_pubkey);
        self.send_instruction_signed(ix, scheduler_pubkey, signer)
            .await
    }

    pub fn stop_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
        provider_payout_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            4,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
                AccountMeta::new(*provider_payout_pubkey, false),
            ],
        )
    }

    pub fn pause_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            2,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

    pub fn resume_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            3,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

    pub fn dispute_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        owner_pubkey: &Pubkey,
    ) -> Instruction {
        make_instruction(
            self.program_id,
            5,
            &[],
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*owner_pubkey, true),
            ],
        )
    }

    pub fn resolve_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        resolver_pubkey: &Pubkey,
        refund_pubkey: &Pubkey,
        provider_payout_pubkey: &Pubkey,
        slash_pubkey: &Pubkey,
        refund_to_buyer: u64,
        provider_payout: u64,
        slash_to_dao: u64,
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&refund_to_buyer.to_le_bytes());
        encoded.extend_from_slice(&provider_payout.to_le_bytes());
        encoded.extend_from_slice(&slash_to_dao.to_le_bytes());
        make_instruction(
            self.program_id,
            6,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*resolver_pubkey, true),
                AccountMeta::new(*refund_pubkey, false),
                AccountMeta::new(*provider_payout_pubkey, false),
                AccountMeta::new(*slash_pubkey, false),
            ],
        )
    }

    pub fn tick_burn_instruction(&self, deployment_pubkey: &Pubkey) -> Instruction {
        make_instruction(
            self.program_id,
            7,
            &[],
            vec![AccountMeta::new(*deployment_pubkey, false)],
        )
    }

    pub fn schedule_pricing_instruction(
        &self,
        deployment_pubkey: &Pubkey,
        governance_pubkey: &Pubkey,
        core_hour: u64,
        ram_gib_hour: u64,
        storage_gib_hour: u64,
        network_mbit_hour: u64,
        effective_at: i64,
    ) -> Instruction {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&core_hour.to_le_bytes());
        encoded.extend_from_slice(&ram_gib_hour.to_le_bytes());
        encoded.extend_from_slice(&storage_gib_hour.to_le_bytes());
        encoded.extend_from_slice(&network_mbit_hour.to_le_bytes());
        encoded.extend_from_slice(&effective_at.to_le_bytes());
        make_instruction(
            self.program_id,
            9,
            &encoded,
            vec![
                AccountMeta::new(*deployment_pubkey, false),
                AccountMeta::new(*governance_pubkey, true),
            ],
        )
    }

    pub fn send_instruction_sync(
        &self,
        _instruction: Instruction,
        _signer_pubkey: &Pubkey,
    ) -> Result<String, SolanaError> {
        Err(SolanaError::Transaction(
            "unsigned Solana transaction submission is not supported".to_string(),
        ))
    }

    pub async fn send_instruction_signed<S: Signer>(
        &self,
        instruction: Instruction,
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        self.send_instructions_signed(core::slice::from_ref(&instruction), signer_pubkey, signer)
            .await
    }

    pub async fn send_instructions_signed<S: Signer>(
        &self,
        instructions: &[Instruction],
        signer_pubkey: &Pubkey,
        signer: &S,
    ) -> Result<String, SolanaError> {
        validate_single_signer(signer_pubkey, instructions)?;
        let recent_blockhash = self.get_latest_blockhash()?;
        let msg = serialize_transaction_message(signer_pubkey, instructions, &recent_blockhash);
        let signature = signer
            .sign(&msg)
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        let tx_bytes =
            serialize_transaction(signer_pubkey, instructions, &recent_blockhash, &signature);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "sendTransaction",
            "params": [
                base64_encode(&tx_bytes),
                { "encoding": "base64", "preflightCommitment": "processed" }
            ]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SolanaError::Rpc("no result in response".to_string()))
    }

    pub fn minimum_balance_for_rent_exemption(&self, size: u64) -> Result<u64, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "getMinimumBalanceForRentExemption",
            "params": [size]
        });
        let resp = self.rpc_call(payload)?;
        resp["result"]
            .as_u64()
            .ok_or_else(|| SolanaError::Rpc("missing rent exemption result".to_string()))
    }

    #[cfg(not(target_os = "none"))]
    pub fn confirm_transaction(&self, signature: &str) -> Result<(), SolanaError> {
        const ATTEMPTS: usize = 30;
        const SLEEP_MS: u64 = 500;

        for _ in 0..ATTEMPTS {
            let payload = json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "getSignatureStatuses",
                "params": [[signature], { "searchTransactionHistory": true }]
            });
            let resp = self.rpc_call(payload)?;
            let statuses = resp["result"]["value"]
                .as_array()
                .ok_or_else(|| SolanaError::Rpc("missing signature status array".to_string()))?;
            if let Some(status) = statuses.first() {
                if status.is_null() {
                    std::thread::sleep(std::time::Duration::from_millis(SLEEP_MS));
                    continue;
                }
                if !status["err"].is_null() {
                    return Err(SolanaError::Transaction(format!(
                        "transaction {signature} failed: {}",
                        status["err"].to_json_string().unwrap_or_default()
                    )));
                }
                if matches!(
                    status["confirmationStatus"].as_str(),
                    Some("confirmed" | "finalized")
                ) {
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(SLEEP_MS));
        }

        Err(SolanaError::Transaction(format!(
            "transaction {signature} was not confirmed after {} seconds",
            (ATTEMPTS as u64 * SLEEP_MS) / 1_000
        )))
    }

    fn get_latest_blockhash(&self) -> Result<Pubkey, SolanaError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "getLatestBlockhash",
            "params": [{ "commitment": "processed" }]
        });
        let resp = self.rpc_call(payload)?;
        let blockhash = resp["result"]["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| SolanaError::Rpc("missing latest blockhash".to_string()))?;
        blockhash
            .parse::<Pubkey>()
            .map_err(|err| SolanaError::Rpc(format!("invalid latest blockhash: {err}")))
    }

    pub async fn post_report_signed<S: Signer>(
        &self,
        deployment_pubkey: &Pubkey,
        provider_pubkey: &Pubkey,
        signer: &S,
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        storage_bytes_used: u64,
        network_bytes_sent: u64,
        container_count: u32,
    ) -> Result<String, SolanaError> {
        let provider_authority = Pubkey::new_from_array(signer.pubkey());
        let instruction = self.report_metrics_instruction(
            deployment_pubkey,
            provider_pubkey,
            &provider_authority,
            cpu_cores_used,
            memory_bytes_used,
            storage_bytes_used,
            network_bytes_sent,
            container_count,
        );
        self.send_instruction_signed(instruction, &provider_authority, signer)
            .await
    }

    pub fn calculate_burn_rate(
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> u64 {
        use crate::types::pricing;
        let cpu_hour = cpu_cores as u64 * pricing::CORE_HOUR;
        let memory_gib = memory_bytes.div_ceil(1024 * 1024 * 1024);
        let memory_hour = memory_gib * pricing::RAM_GIB_HOUR;
        let storage_gib = storage_bytes.div_ceil(1024 * 1024 * 1024);
        let storage_hour = storage_gib * pricing::STORAGE_GIB_HOUR;
        let network_hour = network_mbits as u64 * pricing::NETWORK_MBIT_HOUR;
        (cpu_hour + memory_hour + storage_hour + network_hour) / 3600
    }

    pub fn get_remaining(&self, deployment: &Deployment) -> u64 {
        deployment.deposit.saturating_sub(deployment.spent)
    }

    pub fn is_active(&self, deployment: &Deployment) -> bool {
        matches!(deployment.status, DeploymentStatus::Running)
    }
}

pub fn create_address_with_seed(
    base: &Pubkey,
    seed: &str,
    owner: &Pubkey,
) -> Result<Pubkey, SolanaError> {
    if seed.len() > 32 {
        return Err(SolanaError::Transaction(
            "seed must be 32 bytes or less".to_string(),
        ));
    }

    let mut input = Vec::with_capacity(64 + seed.len());
    input.extend_from_slice(base.as_bytes());
    input.extend_from_slice(seed.as_bytes());
    input.extend_from_slice(owner.as_bytes());
    Ok(Pubkey::new_from_array(edgerun_crypto::sha256(&input)))
}

pub fn create_account_with_seed_instruction(
    payer: &Pubkey,
    new_account: &Pubkey,
    base: &Pubkey,
    seed: &str,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Result<Instruction, SolanaError> {
    if seed.len() > 32 {
        return Err(SolanaError::Transaction(
            "seed must be 32 bytes or less".to_string(),
        ));
    }

    let mut data = Vec::with_capacity(4 + 32 + 8 + seed.len() + 8 + 8 + 32);
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(base.as_bytes());
    data.extend_from_slice(&(seed.len() as u64).to_le_bytes());
    data.extend_from_slice(seed.as_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());
    data.extend_from_slice(&space.to_le_bytes());
    data.extend_from_slice(owner.as_bytes());

    let mut accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*new_account, false),
    ];
    if base != payer {
        accounts.push(AccountMeta::new_readonly(*base));
    }

    Ok(Instruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts,
        data,
    })
}

// Dedicated runtime for HTTP calls
struct HttpRuntime {
    rt: edgerun_rt::Runtime,
}

impl HttpRuntime {
    fn new() -> Self {
        Self {
            rt: edgerun_rt::Builder::new_multi_thread()
                .build()
                .expect("runtime"),
        }
    }

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.rt.block_on(f)
    }
}

pub fn serialize_transaction(
    signer: &Pubkey,
    instructions: &[Instruction],
    recent_blockhash: &Pubkey,
    signature: &[u8; 64],
) -> Vec<u8> {
    let mut buf = Vec::new();
    encode_shortvec_len(1, &mut buf);
    buf.extend_from_slice(signature);
    buf.extend_from_slice(&serialize_transaction_message(
        signer,
        instructions,
        recent_blockhash,
    ));
    buf
}

pub fn serialize_transaction_message(
    signer: &Pubkey,
    instructions: &[Instruction],
    recent_blockhash: &Pubkey,
) -> Vec<u8> {
    let mut buf = Vec::new();
    let (account_keys, readonly_unsigned_count) = compile_account_keys(signer, instructions);

    buf.extend_from_slice(&[1, 0, readonly_unsigned_count]);
    encode_shortvec_len(account_keys.len(), &mut buf);
    for key in &account_keys {
        buf.extend_from_slice(key.as_bytes());
    }
    buf.extend_from_slice(recent_blockhash.as_bytes());
    encode_shortvec_len(instructions.len(), &mut buf);
    for ix in instructions {
        let prog_idx = account_index(&account_keys, ix.program_id);
        buf.push(prog_idx);
        encode_shortvec_len(ix.accounts.len(), &mut buf);
        for meta in &ix.accounts {
            buf.push(account_index(&account_keys, meta.pubkey()));
        }
        encode_shortvec_len(ix.data.len(), &mut buf);
        buf.extend_from_slice(&ix.data);
    }
    buf
}

pub(crate) fn validate_single_signer(
    signer: &Pubkey,
    instructions: &[Instruction],
) -> Result<(), SolanaError> {
    for ix in instructions {
        for meta in &ix.accounts {
            if meta.is_signer() && meta.pubkey() != *signer {
                return Err(SolanaError::Transaction(
                    "instruction requires a signer not provided by this client".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn compile_account_keys(signer: &Pubkey, instructions: &[Instruction]) -> (Vec<Pubkey>, u8) {
    let mut writable_unsigned = Vec::new();
    let mut readonly_unsigned = Vec::new();

    for ix in instructions {
        for meta in &ix.accounts {
            let key = meta.pubkey();
            if key == *signer {
                continue;
            }
            if meta.is_readonly() {
                push_unique(&mut readonly_unsigned, key);
            } else {
                push_unique(&mut writable_unsigned, key);
            }
        }
        push_unique(&mut readonly_unsigned, ix.program_id);
    }

    let readonly_unsigned_count = readonly_unsigned.len() as u8;
    let mut account_keys =
        Vec::with_capacity(1 + writable_unsigned.len() + readonly_unsigned.len());
    account_keys.push(*signer);
    account_keys.extend(writable_unsigned);
    account_keys.extend(readonly_unsigned);
    (account_keys, readonly_unsigned_count)
}

fn push_unique(keys: &mut Vec<Pubkey>, key: Pubkey) {
    if !keys.contains(&key) {
        keys.push(key);
    }
}

fn account_index(account_keys: &[Pubkey], key: Pubkey) -> u8 {
    account_keys
        .iter()
        .position(|candidate| *candidate == key)
        .expect("account key must be compiled before instruction encoding") as u8
}

fn encode_shortvec_len(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            break;
        }
        byte |= 0x80;
        out.push(byte);
    }
}

pub fn base64_encode(data: &[u8]) -> String {
    edgerun_encoding::base64::standard_encode(data)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SolanaError> {
    edgerun_encoding::base64::standard_decode(input)
        .map_err(|err| SolanaError::Rpc(err.to_string()))
}

fn decode_rpc_account_data(resp: &JsonValue) -> Result<Vec<u8>, SolanaError> {
    if resp["result"]["value"].is_null() {
        return Err(SolanaError::AccountNotFound(
            "deployment account".to_string(),
        ));
    }

    let data = if !resp["result"]["value"]["data"].is_null() {
        &resp["result"]["value"]["data"]
    } else {
        &resp["result"]["data"]
    };

    decode_rpc_account_data_value(data)
}

fn decode_rpc_account_data_value(data: &JsonValue) -> Result<Vec<u8>, SolanaError> {
    if let Some(encoded) = data.as_str() {
        return base64_decode(encoded);
    }

    let values = data
        .as_array()
        .ok_or_else(|| SolanaError::Rpc("no data in response".to_string()))?;

    if let Some(encoded) = values.first().and_then(JsonValue::as_str) {
        return base64_decode(encoded);
    }

    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| SolanaError::Rpc("account data byte is not u8".to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_message_uses_solana_wire_shape() {
        let signer = Pubkey::new_from_array([1u8; 32]);
        let account = Pubkey::new_from_array([2u8; 32]);
        let program = Pubkey::new_from_array([3u8; 32]);
        let blockhash = Pubkey::new_from_array([4u8; 32]);
        let instruction = Instruction {
            program_id: program,
            accounts: vec![
                AccountMeta::new(account, false),
                AccountMeta::new(signer, true),
            ],
            data: vec![9, 8, 7],
        };

        let msg = serialize_transaction_message(&signer, &[instruction], &blockhash);

        assert_eq!(&msg[0..3], &[1, 0, 1]);
        assert_eq!(msg[3], 3);
        assert_eq!(&msg[4..36], signer.as_bytes());
        assert_eq!(&msg[36..68], account.as_bytes());
        assert_eq!(&msg[68..100], program.as_bytes());
        assert_eq!(&msg[100..132], blockhash.as_bytes());
        assert_eq!(msg[132], 1);
        assert_eq!(msg[133], 2);
        assert_eq!(msg[134], 2);
        assert_eq!(&msg[135..137], &[1, 0]);
        assert_eq!(msg[137], 3);
        assert_eq!(&msg[138..141], &[9, 8, 7]);
    }

    #[test]
    fn legacy_transaction_prefixes_signature_count_and_message() {
        let signer = Pubkey::new_from_array([1u8; 32]);
        let program = Pubkey::new_from_array([3u8; 32]);
        let blockhash = Pubkey::new_from_array([4u8; 32]);
        let signature = [5u8; 64];
        let instruction = Instruction {
            program_id: program,
            accounts: vec![AccountMeta::new(signer, true)],
            data: vec![1],
        };

        let tx = serialize_transaction(&signer, &[instruction.clone()], &blockhash, &signature);
        let msg = serialize_transaction_message(&signer, &[instruction], &blockhash);

        assert_eq!(tx[0], 1);
        assert_eq!(&tx[1..65], &signature);
        assert_eq!(&tx[65..], msg.as_slice());
    }

    #[test]
    fn deployment_lifecycle_instruction_variants_match_program() {
        let client = DeploymentClient::new_with_program_id(
            "http://127.0.0.1:8899",
            crate::deployment_program_id(),
        )
        .unwrap();
        let deployment = Pubkey::new_from_array([1u8; 32]);
        let owner = Pubkey::new_from_array([2u8; 32]);
        let refund = Pubkey::new_from_array([3u8; 32]);
        let slash = Pubkey::new_from_array([4u8; 32]);

        assert_eq!(client.pause_instruction(&deployment, &owner).data, vec![2]);
        assert_eq!(client.resume_instruction(&deployment, &owner).data, vec![3]);
        assert_eq!(
            client.dispute_instruction(&deployment, &owner).data,
            vec![5]
        );
        assert_eq!(client.tick_burn_instruction(&deployment).data, vec![7]);

        let provider_payout = Pubkey::new_from_array([5u8; 32]);
        let resolve = client.resolve_instruction(
            &deployment,
            &owner,
            &refund,
            &provider_payout,
            &slash,
            500,
            200,
            300,
        );
        assert_eq!(resolve.data[0], 6);
        assert_eq!(
            u64::from_le_bytes(resolve.data[1..9].try_into().unwrap()),
            500
        );
        assert_eq!(
            u64::from_le_bytes(resolve.data[9..17].try_into().unwrap()),
            200
        );
        assert_eq!(
            u64::from_le_bytes(resolve.data[17..25].try_into().unwrap()),
            300
        );
        assert_eq!(resolve.accounts.len(), 5);

        let scheduled = client.schedule_pricing_instruction(
            &deployment,
            &owner,
            10_000,
            5_000,
            1_000,
            2_000,
            1_700_000_000,
        );
        assert_eq!(scheduled.data[0], 9);
        assert_eq!(
            u64::from_le_bytes(scheduled.data[1..9].try_into().unwrap()),
            10_000
        );
        assert_eq!(
            i64::from_le_bytes(scheduled.data[33..41].try_into().unwrap()),
            1_700_000_000
        );
        assert_eq!(scheduled.accounts.len(), 2);

        let provider = Pubkey::new_from_array([6u8; 32]);
        let assign = client.assign_provider_instruction(&deployment, &owner, &provider);
        assert_eq!(assign.data[0], 10);
        assert_eq!(&assign.data[1..33], provider.as_bytes());
        assert_eq!(assign.accounts.len(), 3);
        assert!(assign.accounts[1].is_signer());
    }

    #[test]
    fn get_account_info_null_value_is_account_not_found() {
        let resp = json!({
            "jsonrpc": "2.0",
            "result": { "value": null },
            "id": 1
        });

        match decode_rpc_account_data(&resp) {
            Err(SolanaError::AccountNotFound(label)) => assert_eq!(label, "deployment account"),
            other => panic!("unexpected decode result: {other:?}"),
        }
    }

    #[test]
    fn get_account_info_decodes_base64_data_array() {
        let encoded = base64_encode(&[1, 2, 3, 4]);
        let resp = json!({
            "jsonrpc": "2.0",
            "result": { "value": { "data": [encoded, "base64"] } },
            "id": 1
        });

        assert_eq!(decode_rpc_account_data(&resp).unwrap(), vec![1, 2, 3, 4]);
    }
}
