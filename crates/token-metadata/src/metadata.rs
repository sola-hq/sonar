//! this file contains various helper functions for interacting with token data.
use crate::{
    client::make_rpc_client,
    constants::{TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID},
};
use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, ToPrimitive};
use mpl_token_metadata::accounts::Metadata;
use solana_commitment_config::CommitmentConfig;
use solana_program::program_pack::Pack;
use solana_pubkey::Pubkey;
use sonar_db::{
    models::{Token, TokenMetadata},
    Database, KvStore,
};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, PodStateWithExtensions, StateWithExtensions},
    pod::PodMint,
    state::Mint,
};
use spl_token_metadata_interface::state::TokenMetadata as TokenMetadataExtension;
use std::{
    ops::Div,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::debug;

/// Used to facilitate token data retrieval from the RPC Node, the struct contains
/// mint data for tokens and whether it is a NFT
#[derive(Clone, Debug, Default)]
pub struct PackedTokenData {
    /// The mint associated with this token.
    pub mint: String,
    /// Whether the token is a non-fungible token.
    pub is_nft: bool,
    /// The token account data.
    pub data: Mint,
    /// The token metadata if available from extension
    pub metadata: Option<TokenMetadata>,
}

pub async fn get_token_data(mint: &str) -> Result<PackedTokenData> {
    let client = make_rpc_client();
    let pubkey = Pubkey::from_str(mint).context(format!("Failed to parse mint: {}", mint))?;
    debug!(mint = mint.to_string(), pubkey = pubkey.to_string(), "Fetching mint account");
    let token_account = client
        .get_account_with_commitment(&pubkey, CommitmentConfig::processed())
        .await
        .context(format!("Failed to get mint: {}", mint))?
        .value
        .context(format!("Failed to get mint value: {}", mint))?;

    let (mint_data, token_metadata) = match token_account.owner {
        TOKEN_PROGRAM_ID => {
            let mint = Mint::unpack_from_slice(&token_account.data).expect("Failed to unpack mint");
            (mint, None)
        }
        TOKEN_2022_PROGRAM_ID => {
            let state_with_extensions = StateWithExtensions::<Mint>::unpack(&token_account.data)
                .expect("Failed to unpack mint");
            let mint_data = state_with_extensions.base;
            let pod_state_with_extensions =
                PodStateWithExtensions::<PodMint>::unpack(&token_account.data)
                    .expect("Failed to unpack mint");
            let metadata = pod_state_with_extensions
                .get_variable_len_extension::<TokenMetadataExtension>()
                .ok();
            (mint_data, metadata)
        }
        _ => {
            // should not happen
            return Err(anyhow::anyhow!("Unsupported token program: {}", token_account.owner));
        }
    };
    let is_nft = mint_data.decimals == 0;
    Ok(PackedTokenData {
        mint: mint.to_string(),
        is_nft,
        data: mint_data,
        metadata: token_metadata.map(|metadata| metadata.into()),
    })
}

pub async fn get_mpl_token_metadata(mint: &str) -> Result<TokenMetadata> {
    let client = make_rpc_client();
    let pubkey = Pubkey::from_str(mint).context(format!("Failed to parse mint: {}", mint))?;

    // Find metadata PDA
    let (metadata_pubkey, _) = Metadata::find_pda(&pubkey);
    debug!(
        mint = mint.to_string(),
        metadata_pubkey = metadata_pubkey.to_string(),
        "Fetching MPL metadata"
    );
    let account = client
        .get_account_with_commitment(&metadata_pubkey, CommitmentConfig::processed())
        .await
        .context(format!("Failed to get metadata account: {}", mint))?
        .value
        .context(format!("Failed to get metadata account value: {}", mint))?;

    let metadata = Metadata::from_bytes(&account.data).expect("Failed to unpack metadata");
    let token_metadata = TokenMetadata::from(metadata);
    Ok(token_metadata)
}

/// Trait to extend TokenMetadata with additional functionality
pub trait TokenMetadataExt {
    /// Get a field from either the primary or fallback metadata with a default value
    fn get_field_with_fallback<T>(
        primary: &Option<Self>,
        fallback: &Option<Self>,
        getter: fn(&Self) -> T,
        default: T,
    ) -> T
    where
        Self: Sized;
}

impl TokenMetadataExt for TokenMetadata {
    fn get_field_with_fallback<T>(
        primary: &Option<Self>,
        fallback: &Option<Self>,
        getter: fn(&Self) -> T,
        default: T,
    ) -> T {
        if let Some(metadata) = primary {
            return getter(metadata);
        }
        if let Some(metadata) = fallback {
            return getter(metadata);
        }
        default
    }
}

pub fn pack_token_metadata(
    packed: &PackedTokenData,
    token_metadata: &Option<TokenMetadata>,
) -> Token {
    let pack_token_metadata = &packed.metadata;
    let decimals = packed.data.decimals;
    let supply_decimal = BigDecimal::from(packed.data.supply);
    let supply = supply_decimal
        .div(10_f64.powi(decimals as i32))
        .to_f64()
        .expect("Failed to convert to f64");

    Token {
        retrieval_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time only moves forward")
            .as_secs(),
        is_nft: packed.is_nft,
        token: packed.mint.to_string(),
        update_authority: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.update_authority.to_string(),
            "".to_string(),
        ),
        name: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.name.clone(),
            "".to_string(),
        ),
        symbol: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.symbol.clone(),
            "".to_string(),
        ),
        uri: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.uri.clone(),
            "".to_string(),
        ),
        decimals,
        supply,
        seller_fee_basis_points: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.seller_fee_basis_points,
            0,
        ),
        primary_sale_happened: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.primary_sale_happened,
            false,
        ),
        is_mutable: TokenMetadata::get_field_with_fallback(
            pack_token_metadata,
            token_metadata,
            |t| t.is_mutable,
            false,
        ),
    }
}

pub async fn get_token_metadata_with_data(
    mint: &str,
    kv_store: &Arc<KvStore>,
    db: &Arc<Database>,
) -> Result<Token> {
    if let Some(token) =
        kv_store.get_token(mint).await.context("Failed to get token from kv store")?
    {
        return Ok(token);
    }

    if let Some(token) = db.get_token(mint).await.context("Failed to get token from db")? {
        kv_store.set_token(mint, &token).await.context("Failed to set token in kv store")?;
        return Ok(token);
    }

    let pack_token = get_token_data(mint).await.context("Failed to get token data from rpc")?;
    let token_metadata = if let Some(metadata) = &pack_token.metadata {
        Some(metadata.clone())
    } else {
        // Fall back to MPL metadata if extension metadata is not available
        get_mpl_token_metadata(mint).await.ok()
    };

    let token = pack_token_metadata(&pack_token, &token_metadata);

    db.insert_token(&token).await.context("Failed to insert token into db")?;
    kv_store.set_token(mint, &token).await.context("Failed to set token in kv store")?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn test_get_usdc_data() {
        dotenv().ok();
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let pack_token = get_token_data(mint).await.expect("failed to get token data");
        let token = pack_token.data;
        assert_eq!(token.decimals, 6, "USDC has 6 decimals");
        assert!(token.supply > 10, "USDC has a supply of more than 10");
        assert!(token.is_initialized, "USDC is initialized");
        assert!(token.mint_authority.is_some(), "USDC has a mint authority");
        assert!(token.freeze_authority.is_some(), "USDC has a freeze authority");
        assert!(!pack_token.is_nft, "USDC is not an NFT");
    }

    #[tokio::test]
    async fn test_get_trump_data() {
        dotenv().ok();
        let mint = "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN";
        let pack_token = get_token_data(mint).await.expect("failed to get token data");
        let token = pack_token.data;
        assert_eq!(token.decimals, 6, "Trump is an NFT");
        assert!(token.is_initialized, "Trump is initialized");
        assert!(token.mint_authority.is_none(), "Trump does not have a mint authority");
        assert!(token.freeze_authority.is_none(), "Trump does not have a freeze authority");
        assert!(!pack_token.is_nft, "Trump is not an NFT");
    }

    #[tokio::test]
    async fn test_get_mad_lads_data() {
        dotenv().ok();

        // Mad Lads #6958
        let mint = "4HT4aX6wq3SsKqey5FXaqMaZ9uEN3rinNWvhGzWKotjt";
        let pack_token = get_token_data(mint).await.expect("failed to get token data");
        let token = pack_token.data;
        assert_eq!(token.decimals, 0, "Mad Lads is an NFT");
        assert!(token.is_initialized, "Mad Lads is initialized");
        assert_eq!(
            token.mint_authority.unwrap().to_string(),
            "2nKAsyXns15MFnk5oLMFzz252pFgk9GhqjPGhPoLXESJ"
        );
        assert_eq!(
            token.freeze_authority.unwrap().to_string(),
            "2nKAsyXns15MFnk5oLMFzz252pFgk9GhqjPGhPoLXESJ"
        );
    }

    #[tokio::test]
    async fn test_get_usdc_metadata() {
        dotenv().ok();
        let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let token_metadata =
            get_mpl_token_metadata(mint).await.expect("Failed to get token metadata");

        assert_eq!(token_metadata.mint.to_string(), mint.to_string());
        assert_eq!(token_metadata.name, "USD Coin");
        assert_eq!(token_metadata.symbol, "USDC");
        assert_eq!(token_metadata.uri, "");
    }

    #[tokio::test]
    async fn test_get_trump_metadata() {
        dotenv().ok();

        let mint = "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN";
        let token_metadata =
            get_mpl_token_metadata(mint).await.expect("failed to get token metadata");
        assert_eq!(token_metadata.mint.to_string(), mint.to_string());
        assert_eq!(token_metadata.name, "OFFICIAL TRUMP");
        assert_eq!(token_metadata.symbol, "TRUMP");
        assert_eq!(
            token_metadata.uri,
            "https://arweave.net/cSCP0h2n1crjeSWE9KF-XtLciJalDNFs7Vf-Sm0NNY0"
        );
        assert_eq!(token_metadata.seller_fee_basis_points, 0);
        assert!(!token_metadata.primary_sale_happened);
        assert!(!token_metadata.is_mutable);
        assert_eq!(
            token_metadata.update_authority.to_string(),
            "5e2qRc1DNEXmyxP8qwPwJhRWjef7usLyi7v5xjqLr5G7"
        );
        assert_eq!(token_metadata.mint.to_string(), "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN");
    }

    #[tokio::test]
    async fn test_get_mad_lads_metadata() {
        dotenv().ok();

        let mint = "4HT4aX6wq3SsKqey5FXaqMaZ9uEN3rinNWvhGzWKotjt";
        let token_metadata =
            get_mpl_token_metadata(mint).await.expect("failed to get token metadata");
        assert_eq!(token_metadata.mint.to_string(), mint.to_string());
        assert_eq!(token_metadata.name, "Mad Lads #6958");
        assert_eq!(token_metadata.symbol, "MAD");
        assert_eq!(token_metadata.uri, "https://madlads.s3.us-west-2.amazonaws.com/json/6958.json");

        assert!(token_metadata.primary_sale_happened);
        assert!(token_metadata.is_mutable);
        //
        assert!(token_metadata.seller_fee_basis_points > 0);
        assert!(!token_metadata.update_authority.is_empty());
    }

    #[tokio::test]
    async fn test_pack_token_metadata() {
        dotenv().ok();
        // token 2022
        {
            // token with extension metadata
            let mint = "4GwjPsmf3HuUVzFU7W9g75VBZkvezHmFZLYv1BzzonTx";
            let pack_token = get_token_data(mint).await.expect("Failed to get token data");
            assert!(pack_token.data.is_initialized);
            let _ = get_mpl_token_metadata(mint).await.is_err();
        }

        // 6J7mUbPXcAASzmG4k3umUnT1zaSw97WwduJM2aKJCeiF
        {
            // token without extension metadata
            let mint = "6J7mUbPXcAASzmG4k3umUnT1zaSw97WwduJM2aKJCeiF";
            let pack_token = get_token_data(mint).await.expect("Failed to get token data");
            assert!(pack_token.data.is_initialized);
            let _ = get_mpl_token_metadata(mint).await.is_err();
        }

        // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        {
            // token with spl token metadata
            let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
            let pack_token = get_token_data(mint).await.expect("Failed to get token data");
            assert!(pack_token.data.is_initialized);
            assert_eq!(pack_token.mint.to_string(), mint.to_string());
            let token_metadata =
                get_mpl_token_metadata(mint).await.expect("Failed to get token metadata");
            assert_eq!(token_metadata.name, "USD Coin");
            assert_eq!(token_metadata.symbol, "USDC");
            assert_eq!(token_metadata.uri, "");
            assert_eq!(token_metadata.seller_fee_basis_points, 0);
            assert!(!token_metadata.primary_sale_happened);
            assert!(token_metadata.is_mutable);
        }
    }
}
