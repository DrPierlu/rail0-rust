/// Static metadata for a single stablecoin on a specific chain.
#[derive(Debug, Clone, Copy)]
pub struct StablecoinInfo {
    /// Checksummed contract address.
    pub address: &'static str,
    /// Number of decimal places (6 for USDC/USDT, 18 for DAI).
    pub decimals: u8,
    /// Supports `transferWithAuthorization` (EIP-3009 / Circle standard). Required by RAIL0.
    pub eip3009: bool,
    /// Supports `permit` (EIP-2612). Not used by RAIL0 but useful for other integrations.
    pub eip2612: bool,
    /// Bridge-wrapped variant that may not support either auth extension.
    pub bridged: bool,
}

/// Chain-level metadata with its token registry.
pub struct ChainStablecoins {
    pub chain_id: u64,
    /// Slice of `(symbol, info)` pairs.
    pub tokens: &'static [(&'static str, StablecoinInfo)],
}

/// A simplified token view returned by [`eip3009_tokens`] and [`eip2612_tokens`].
#[derive(Debug, Clone)]
pub struct StablecoinToken {
    pub symbol: &'static str,
    pub address: &'static str,
    pub decimals: u8,
}

// ================================================================
//  Static registry
// ================================================================

const fn t(
    address: &'static str,
    decimals: u8,
    eip3009: bool,
    eip2612: bool,
    bridged: bool,
) -> StablecoinInfo {
    StablecoinInfo { address, decimals, eip3009, eip2612, bridged }
}

static ETHEREUM_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",  t("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6,  true,  false, false)),
    ("EURC",  t("0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c", 6,  true,  false, false)),
    ("PYUSD", t("0x6c3ea9036406852006290770BEdFcAbA0e23A0e8", 6,  true,  false, false)),
    ("USDT",  t("0xdAC17F958D2ee523a2206206994597C13D831ec7", 6,  false, false, false)),
    ("DAI",   t("0x6B175474E89094C44Da98b954EedeAC495271d0F", 18, false, true,  false)),
];

static BASE_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",  t("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6, true,  false, false)),
    ("EURC",  t("0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1adb42", 6, true,  false, false)),
    ("USDbC", t("0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA", 6, false, false, true)),
];

static POLYGON_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",   t("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", 6,  true,  false, false)),
    ("USDC.e", t("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6,  true,  false, true)),
    ("USDT",   t("0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6,  false, false, false)),
    ("DAI",    t("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", 18, false, false, false)),
];

static ARBITRUM_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",   t("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6,  true,  false, false)),
    ("USDC.e", t("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8", 6,  false, false, true)),
    ("USDT",   t("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6,  false, false, false)),
    ("DAI",    t("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", 18, false, true,  false)),
];

static OPTIMISM_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",   t("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85", 6,  true,  false, false)),
    ("USDC.e", t("0x7F5c764cBc14f9669B88837ca1490cCa17c31607", 6,  false, false, true)),
    ("USDT",   t("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58", 6,  false, false, false)),
    ("DAI",    t("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", 18, false, true,  false)),
];

static AVALANCHE_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC",   t("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E", 6, true,  false, false)),
    ("USDC.e", t("0xA7D7079b0FEaD91F3e65f86E8915Cb59c1a4C664", 6, false, false, true)),
    ("USDT",   t("0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7", 6, false, false, false)),
];

static CELO_TOKENS: &[(&str, StablecoinInfo)] = &[
    ("USDC", t("0xcebA9300f2b948710d2De3250b7Ad3e4aFb0e50a", 6,  true, false, false)),
    ("cUSD", t("0x765DE816845861e75A25fCA122bb6898B8B1282a", 18, true, false, false)),
    ("cEUR", t("0xD8763CBa276a3738E6DE85b4b3bF5FDed6D6cA73", 18, true, false, false)),
];

static ETHEREUM: ChainStablecoins = ChainStablecoins { chain_id: 1,     tokens: ETHEREUM_TOKENS };
static BASE:     ChainStablecoins = ChainStablecoins { chain_id: 8453,  tokens: BASE_TOKENS };
static POLYGON:  ChainStablecoins = ChainStablecoins { chain_id: 137,   tokens: POLYGON_TOKENS };
static ARBITRUM: ChainStablecoins = ChainStablecoins { chain_id: 42161, tokens: ARBITRUM_TOKENS };
static OPTIMISM: ChainStablecoins = ChainStablecoins { chain_id: 10,    tokens: OPTIMISM_TOKENS };
static AVALANCHE:ChainStablecoins = ChainStablecoins { chain_id: 43114, tokens: AVALANCHE_TOKENS };
static CELO:     ChainStablecoins = ChainStablecoins { chain_id: 42220, tokens: CELO_TOKENS };

// ================================================================
//  Public API
// ================================================================

/// Returns the registry entry for a chain, or `None` if the chain is unknown.
///
/// Supported chain names: `"ethereum"`, `"base"`, `"polygon"`, `"arbitrumOne"`,
/// `"optimism"`, `"avalanche"`, `"celo"`.
pub fn chain_info(chain: &str) -> Option<&'static ChainStablecoins> {
    match chain {
        "ethereum"    => Some(&ETHEREUM),
        "base"        => Some(&BASE),
        "polygon"     => Some(&POLYGON),
        "arbitrumOne" => Some(&ARBITRUM),
        "optimism"    => Some(&OPTIMISM),
        "avalanche"   => Some(&AVALANCHE),
        "celo"        => Some(&CELO),
        _             => None,
    }
}

/// Returns all tokens on the given chain that support `transferWithAuthorization` (EIP-3009).
/// These are the tokens compatible with RAIL0.
///
/// ```
/// let tokens = rail0::eip3009_tokens("base");
/// // tokens[0] => StablecoinToken { symbol: "USDC", address: "0x833...", decimals: 6 }
/// ```
pub fn eip3009_tokens(chain: &str) -> Vec<StablecoinToken> {
    chain_info(chain)
        .map(|c| {
            c.tokens
                .iter()
                .filter(|(_, t)| t.eip3009)
                .map(|(sym, t)| StablecoinToken { symbol: sym, address: t.address, decimals: t.decimals })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns all tokens on the given chain that support `permit` (EIP-2612).
pub fn eip2612_tokens(chain: &str) -> Vec<StablecoinToken> {
    chain_info(chain)
        .map(|c| {
            c.tokens
                .iter()
                .filter(|(_, t)| t.eip2612)
                .map(|(sym, t)| StablecoinToken { symbol: sym, address: t.address, decimals: t.decimals })
                .collect()
        })
        .unwrap_or_default()
}
