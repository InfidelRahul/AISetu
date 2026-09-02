//! A deterministic test case and a Needle-backed test case both produce
//! validated structured output through the same IntelligenceEngine abstraction.

use aisetu_intelligence::{
    DeterministicEngine, IntelligenceContext, IntelligenceEngine, IntelligenceInput, JsonSchema,
    NeedleEngine,
};

fn schema() -> JsonSchema {
    JsonSchema::object(&["ok"], vec![("ok", JsonSchema::boolean())])
}

async fn run(engine: &dyn IntelligenceEngine, text: &str) -> aisetu_core::Result<bool> {
    let out = engine
        .infer(
            &IntelligenceInput::new(text),
            &schema(),
            &IntelligenceContext::default(),
        )
        .await?;
    schema().validate(&out.value)?;
    assert!(out.confidence > 0.0);
    Ok(out.value["ok"].as_bool().unwrap())
}

#[tokio::test]
async fn both_engines_same_contract() {
    let det = DeterministicEngine;
    let needle = NeedleEngine::new();
    assert!(run(&det, r#"{"ok": true}"#).await.unwrap());
    assert!(run(&needle, r#"prefix {"ok": true} suffix"#).await.unwrap());
    assert_eq!(det.name(), "deterministic");
    assert_eq!(needle.name(), "needle");
}
