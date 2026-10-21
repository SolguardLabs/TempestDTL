# Operación y observabilidad

## Entorno reproducible

Use Rust 1.96 y Node.js 24. `Cargo.lock` fija dependencias. No se necesitan RPC, credenciales ni servicios persistentes para los escenarios incluidos. En un entorno con ambas herramientas:

```bash
bash scripts/ci.sh
cargo run --quiet --locked -- normal
cargo run --quiet --locked -- queued
cargo run --quiet --locked -- pause
```

CI ejecuta `cargo fmt --check`, `cargo build --all-targets --locked`, pruebas Rust, Clippy con advertencias como errores y pruebas Node sobre la CLI. `scripts/generate-catalog.mjs` mantiene el catálogo generado; cualquier actualización del catálogo debe revisarse junto a los límites efectivos de riesgo.

## Secuencia de revisión

1. Conserve la configuración de pools y la observación de cada mercado antes de admitir una ruta.
2. Al crear un ticket, registre `slot`, digest, ventana de expiración, hops e importes por activo. Asocie `client_order_id` en la capa integradora.
3. Observe `mode`, eventos de mercado y `operations.market_pipeline` para conocer la carga pendiente por mercado/dirección.
4. En cada recibo, reconcilie débito de entrada, crédito de salida, reservas de pools y entradas del diario.
5. Al comparar dos ejecuciones, use `state_digest` junto con el catálogo y los parámetros de escenario; un digest distinto exige investigar el primer evento divergente.

Los escenarios de CLI reinician el estado. Una operación real integrada debe aportar transacciones e idempotencia; reejecutar un comando de ejemplo no reanuda una orden previa.

## Identidad de publicación

`main`, `production` y `v1.0.0` se fijan al mismo commit. `release-integrity` comprueba esa identidad y los documentos necesarios. La release identifica el artefacto de código, no un despliegue de red.
