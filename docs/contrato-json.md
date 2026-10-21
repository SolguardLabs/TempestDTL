# Contrato JSON de escenarios

Cada comando de la CLI imprime un único objeto `ScenarioReport`. Los campos principales son `scenario`, `network_id`, `slot`, `mode`, `route_admitted`, `rejection`, `ticket`, `receipt`, `balances`, `pools`, `moves`, `events`, `journal_count`, `state_digest` y `operations`. Un `ticket` o `receipt` ausente se representa como `null`. Los importes se serializan como enteros de unidades internas de cada activo; no convierta saldos grandes a coma flotante sin comprobar el rango seguro del consumidor.

## Operaciones pendientes

`operations` incluye `slot`, `mode`, `prepared_tickets`, `settled_tickets`, `pool_count`, `event_count` y `market_pipeline`. Cada línea de `market_pipeline` conserva `market`, `asset_in`, `asset_out`, `prepared_routes`, `input_units`, `output_units` y `oldest_admission_slot`.

Ejemplo abreviado que muestra solo la línea `SOL-USDC` del escenario `queued`:

```json
{
  "operations": {
    "slot": 1,
    "mode": "open",
    "prepared_tickets": 2,
    "settled_tickets": 0,
    "pool_count": 2,
    "event_count": 2,
    "market_pipeline": [
      {
        "market": "SOL-USDC",
        "asset_in": "USDC",
        "asset_out": "SOL",
        "prepared_routes": 2,
        "input_units": 17000,
        "output_units": 841,
        "oldest_admission_slot": 1
      }
    ]
  }
}
```

El ejemplo omite la otra línea y otros campos del escenario; no reproduce el orden completo del array. Para la salida íntegra, ejecute `cargo run --quiet --locked -- queued`; el orden de líneas sigue la clave `(mercado, origen, destino)`. `operations` es una instantánea de lectura: no reserva capacidad y no cambia el estado de los tickets. Los consumidores deben conservar la versión de la biblioteca y el catálogo usados para interpretar campos y límites.
