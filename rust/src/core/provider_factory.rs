//! Canonical provider factory.
//!
//! All call sites (CLI, Tauri desktop shell, legacy native UI) must construct
//! provider instances through [`instantiate`] so that adding a new provider
//! only requires editing `ProviderId`, the matching `providers/` module, and
//! this one match arm.

use super::{Provider, ProviderId};
use crate::providers::{
    AbacusProvider, AlibabaProvider, AmpProvider, AntigravityProvider, AugmentProvider,
    ClaudeProvider, CodexProvider, CopilotProvider, CursorProvider, FactoryProvider,
    GeminiProvider, InfiniProvider, JetBrainsProvider, KiloProvider, KimiK2Provider, KimiProvider,
    KiroProvider, MiniMaxProvider, NanoGPTProvider, OllamaProvider, OpenCodeGoProvider,
    OpenCodeProvider, OpenRouterProvider, PerplexityProvider, SyntheticProvider, VertexAIProvider,
    WarpProvider, ZaiProvider,
};

/// Instantiate the concrete [`Provider`] implementation for a given [`ProviderId`].
///
/// Exhaustive over [`ProviderId`]: adding a new variant is a compile error until
/// the corresponding provider type is wired in below.
pub fn instantiate(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::Claude => Box::new(ClaudeProvider::new()),
        ProviderId::Codex => Box::new(CodexProvider::new()),
        ProviderId::Cursor => Box::new(CursorProvider::new()),
        ProviderId::Gemini => Box::new(GeminiProvider::new()),
        ProviderId::Copilot => Box::new(CopilotProvider::new()),
        ProviderId::Antigravity => Box::new(AntigravityProvider::new()),
        ProviderId::Factory => Box::new(FactoryProvider::new()),
        ProviderId::Zai => Box::new(ZaiProvider::new()),
        ProviderId::Kiro => Box::new(KiroProvider::new()),
        ProviderId::VertexAI => Box::new(VertexAIProvider::new()),
        ProviderId::Augment => Box::new(AugmentProvider::new()),
        ProviderId::MiniMax => Box::new(MiniMaxProvider::new()),
        ProviderId::OpenCode => Box::new(OpenCodeProvider::new()),
        ProviderId::Kimi => Box::new(KimiProvider::new()),
        ProviderId::KimiK2 => Box::new(KimiK2Provider::new()),
        ProviderId::Amp => Box::new(AmpProvider::new()),
        ProviderId::Warp => Box::new(WarpProvider::new()),
        ProviderId::Ollama => Box::new(OllamaProvider::new()),
        ProviderId::OpenRouter => Box::new(OpenRouterProvider::new()),
        ProviderId::Synthetic => Box::new(SyntheticProvider::new()),
        ProviderId::JetBrains => Box::new(JetBrainsProvider::new()),
        ProviderId::Alibaba => Box::new(AlibabaProvider::new()),
        ProviderId::NanoGPT => Box::new(NanoGPTProvider::new()),
        ProviderId::Infini => Box::new(InfiniProvider::default()),
        ProviderId::Perplexity => Box::new(PerplexityProvider::new()),
        ProviderId::Abacus => Box::new(AbacusProvider::new()),
        ProviderId::OpenCodeGo => Box::new(OpenCodeGoProvider::new()),
        ProviderId::Kilo => Box::new(KiloProvider::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_id_is_instantiable() {
        for &id in ProviderId::all() {
            let provider = instantiate(id);
            assert_eq!(
                provider.id(),
                id,
                "factory returned wrong provider for {id}"
            );
        }
    }
}
