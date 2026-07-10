# Seguridad

## Modelo

TempestDTL asume que los operadores de ruta, pools y oraculos internos son
componentes autorizados por el protocolo. Las decisiones de admision deben
mantener consistencia entre precio observado, liquidez disponible, limites de
desviacion, TTL de tickets y modo operativo del router.

## Invariantes Esperadas

- Ninguna ruta se admite si el router esta en pausa operativa.
- Los cambios bruscos de precio o profundidad activan protecciones de mercado.
- La reactivacion exige una ventana de estabilidad antes de admitir nuevas rutas.
- La liquidacion de tickets debe mantener balances no negativos.
- Las reservas de pool no pueden quedar por debajo de los minimos configurados.
- Los digests de auditoria deben ser deterministas para escenarios iguales.

## Validaciones

La suite local ejecuta:

- tests unitarios Rust para arithmetic, pools, oracle, pausa y settlement;
- tests de integracion JavaScript sobre el CLI;
- clippy con advertencias tratadas como errores;
- formateo Rust en CI.

## Alcance De Revision

El alcance recomendado incluye:

- cotizacion y admision de rutas;
- transiciones del controlador de pausa;
- actualizacion de oraculos y perfiles de riesgo;
- liquidacion y accounting multi-activo;
- serializacion JSON de escenarios;
- catalogo de perfiles y parametros por mercado.

## Reporte Interno

Un reporte debe incluir impacto economico, pasos de reproduccion, patch
propuesto y pruebas nuevas. No incluir datos sensibles, claves, endpoints
privados ni artefactos de infraestructura fuera del repositorio.
