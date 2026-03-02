# UPI Switch Rust Prototype

Backend-only Rust prototype of a UPI-style payment switch with idempotency, deterministic mock banks, timeout handling, and compensating reversal.

## Run

```bash
cargo run
```

Server starts on `http://localhost:8080`.

## API

### `POST /pay`

Request JSON:

```json
{
  "transaction_id": "550e8400-e29b-41d4-a716-446655440000",
  "payer_vpa": "raj@okaxis",
  "payee_vpa": "chaiwala@ybl",
  "amount_paise": 1000,
  "payer_bank_ifsc": "UTIB0000001",
  "payee_bank_ifsc": "YESB0000001",
  "timestamp_ms": 1710000000000
}
```

Response JSON:

```json
{
  "transaction_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "SUCCESS",
  "rrn": "RRN-550e8400-CREDIT-YesBank",
  "error_code": null,
  "processing_time_ms": 402
}
```

`status` values: `SUCCESS`, `FAILED`, `TIMEOUT`, `DUPLICATE`

HTTP status mapping:

- `SUCCESS` -> `200`
- `FAILED` -> `402`
- `TIMEOUT` -> `504`
- `DUPLICATE` -> `200`

## Demo Requests

### 1. Success

```bash
curl -X POST http://localhost:8080/pay \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_id": "550e8400-e29b-41d4-a716-446655440000",
    "payer_vpa": "raj@okaxis",
    "payee_vpa": "chaiwala@ybl",
    "amount_paise": 1000,
    "payer_bank_ifsc": "UTIB0000001",
    "payee_bank_ifsc": "YESB0000001",
    "timestamp_ms": 1893456000000
  }'
```

### 2. Duplicate

Run the same request again with the same `transaction_id`. Expected `status: "DUPLICATE"`.

### 3. Deterministic Credit Failure

```bash
curl -X POST http://localhost:8080/pay \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_id": "660e8400-e29b-41d4-a716-446655440000",
    "payer_vpa": "raj@okaxis",
    "payee_vpa": "merchant-creditfail@ybl",
    "amount_paise": 1000,
    "payer_bank_ifsc": "UTIB0000001",
    "payee_bank_ifsc": "YESB0000001",
    "timestamp_ms": 1893456000000
  }'
```

Expected `status: "FAILED"` with an error code containing `CREDIT_FAILED`.

### 4. Deterministic Timeout

```bash
curl -X POST http://localhost:8080/pay \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_id": "770e8400-e29b-41d4-a716-446655440000",
    "payer_vpa": "raj@okaxis",
    "payee_vpa": "merchant-slow@ybl",
    "amount_paise": 1000,
    "payer_bank_ifsc": "UTIB0000001",
    "payee_bank_ifsc": "YESB0000001",
    "timestamp_ms": 1893456000000
  }'
```

Expected `status: "TIMEOUT"`.

## Test

```bash
cargo test
```

## Notes

- Idempotency store is in-memory (`DashMap`) and process-local.
- This is a prototype; no settlement, no real bank integration, and no persistent storage.
