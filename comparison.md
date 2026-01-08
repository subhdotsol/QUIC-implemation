# HTTP vs QUIC: A Complete Comparison

Understanding when to use traditional HTTP/1.1, HTTP/2, or QUIC/HTTP/3.

---

## Protocol Stack Comparison

```
HTTP/1.1 & HTTP/2              HTTP/3 (QUIC)
┌─────────────────┐            ┌─────────────────┐
│  HTTP/1.1 or 2  │            │     HTTP/3      │
├─────────────────┤            ├─────────────────┤
│      TLS 1.2+   │            │      QUIC       │
├─────────────────┤            │   (TLS 1.3      │
│       TCP       │            │   built-in)     │
├─────────────────┤            ├─────────────────┤
│       IP        │            │       UDP       │
└─────────────────┘            ├─────────────────┤
                               │       IP        │
                               └─────────────────┘
```

---

## Head-of-Line Blocking

### The Problem

```
HTTP/2 over TCP (one packet lost):

Stream 1: [Data] [Data] [LOST] [Data] [Data]
Stream 2: [Data] [Data]   ↓    [Data] [Data]  ← Blocked!
Stream 3: [Data] [Data]   ↓    [Data] [Data]  ← Blocked!
                          │
                    All streams wait
                    for retransmission
```

### QUIC Solution

```
HTTP/3 over QUIC (one packet lost):

Stream 1: [Data] [Data] [LOST] ──→ waits for retransmission
Stream 2: [Data] [Data] [Data] [Data] ✓ continues!
Stream 3: [Data] [Data] [Data] [Data] ✓ continues!

Only the affected stream is blocked!
```

---

## Connection Establishment

### HTTP/1.1 + TLS (3 Round Trips)

```
Client                          Server
   │                               │
   ├── SYN ────────────────────────┤ ┐
   │◄─────────────────── SYN-ACK ──┤ │ TCP Handshake
   ├── ACK ────────────────────────┤ ┘
   │                               │
   ├── ClientHello ────────────────┤ ┐
   │◄─────────── ServerHello ──────┤ │ TLS Handshake
   ├── Finished ───────────────────┤ │
   │◄──────────────── Finished ────┤ ┘
   │                               │
   ├── HTTP Request ───────────────┤  Finally!
   │                               │

   Total: 3 RTT before first request
```

### HTTP/3 + QUIC (1 Round Trip)

```
Client                          Server
   │                               │
   ├── Initial (ClientHello) ──────┤ ┐
   │◄── Initial (ServerHello) ─────┤ │ Combined!
   │◄── Handshake + 1-RTT data ────┤ ┘
   │                               │
   ├── HTTP/3 Request ─────────────┤  Done!
   │                               │

   Total: 1 RTT (or 0-RTT for resumed connections!)
```

---

## Feature Comparison

| Feature | HTTP/1.1 | HTTP/2 | HTTP/3 (QUIC) |
|---------|----------|--------|---------------|
| **Transport** | TCP | TCP | UDP |
| **Encryption** | Optional | Optional* | Mandatory TLS 1.3 |
| **Multiplexing** | ❌ | ✅ | ✅ |
| **Head-of-Line Blocking** | Per-connection | Per-connection | Per-stream only |
| **Connection Handshake** | 3 RTT | 2-3 RTT | 1 RTT (0-RTT possible) |
| **Connection Migration** | ❌ | ❌ | ✅ |
| **Header Compression** | ❌ | HPACK | QPACK |
| **Server Push** | ❌ | ✅ | ✅ |

*HTTP/2 is practically always used with TLS

---

## Connection Migration

### Traditional HTTP (TCP)

```
WiFi Network                    Cellular Network
┌──────────┐                    ┌──────────┐
│  Client  │                    │  Client  │
│ IP: A.A  │ ──── switch ────→  │ IP: B.B  │
└──────────┘     network        └──────────┘
     │                               │
     │ TCP connection                │ NEW TCP connection
     │ (source IP: A.A)              │ (source IP: B.B)
     │                               │
     ▼                               ▼
   SERVER                          SERVER
   
   ❌ Old connection dies
   ❌ New connection requires full handshake
   ❌ All in-flight data lost
```

### QUIC

```
WiFi Network                    Cellular Network
┌──────────┐                    ┌──────────┐
│  Client  │                    │  Client  │
│ IP: A.A  │ ──── switch ────→  │ IP: B.B  │
│ ConnID:X │     network        │ ConnID:X │
└──────────┘                    └──────────┘
     │                               │
     │ QUIC connection               │ SAME QUIC connection
     │ (identified by               │ (still identified by
     │  Connection ID X)            │  Connection ID X!)
     │                               │
     ▼                               ▼
   SERVER                          SERVER
   
   ✅ Connection survives!
   ✅ No re-handshake needed
   ✅ No data loss
```

---

## When to Use What?

### Use HTTP/1.1 When:
- Legacy system compatibility required
- Simple request-response patterns
- Debugging (easier to read in plaintext)
- Proxies/firewalls don't support HTTP/2+

### Use HTTP/2 When:
- Need multiplexing over stable connections
- Modern web applications with many assets
- gRPC (built on HTTP/2)
- Wide client/server support needed

### Use HTTP/3 (QUIC) When:
- **High latency networks** — 0-RTT and 1-RTT connections
- **Lossy networks (mobile, WiFi)** — No head-of-line blocking
- **Mobile apps** — Connection migration when switching networks
- **Real-time applications** — Lower latency matters
- **Video streaming** — Each stream independent
- **API services** — Faster connection establishment

---

## Performance Scenarios

### Scenario 1: Stable Fiber Connection
```
HTTP/2: ████████████████████ 100ms
HTTP/3: ████████████████████ 100ms (similar, slight 0-RTT advantage)

Winner: Tie (TCP works fine, QUIC has minimal overhead benefit)
```

### Scenario 2: Mobile Network with Packet Loss
```
HTTP/2: ████████████████████████████████████ 400ms (retransmits block all)
HTTP/3: ████████████████████ 150ms (only affected stream waits)

Winner: HTTP/3 (no head-of-line blocking)
```

### Scenario 3: User Switching WiFi → Cellular
```
HTTP/2: ████████████████████████████████████████ 500ms+ (new connection)
HTTP/3: ████████████████ 50ms (connection migrates seamlessly)

Winner: HTTP/3 (connection migration)
```

### Scenario 4: Cold Start (First Request)
```
HTTP/2: ████████████████████████████ (3 RTT handshake)
HTTP/3: ██████████████ (1 RTT handshake)

Winner: HTTP/3 (faster handshake)
```

### Scenario 5: Warm Start (Resumed Connection)
```
HTTP/2: ████████████████████ (1-2 RTT with TLS session resumption)
HTTP/3: ████████ (0-RTT! Data sent with first packet)

Winner: HTTP/3 (0-RTT resumption)
```

---

## Real-World Adoption

| Company | Use Case |
|---------|----------|
| **Google** | YouTube, Gmail, Search — pioneered QUIC |
| **Cloudflare** | CDN edges — 25% of web traffic |
| **Facebook** | Mobile apps — connection migration |
| **Uber** | Mobile APIs — network switching |
| **Discord** | Real-time chat — low latency |

---

## Summary: Decision Matrix

```
┌─────────────────────────────────────────────────────────────────┐
│                    Should I use HTTP/3?                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                  ┌───────────────────────┐
                  │ Is latency critical?  │
                  └───────────────────────┘
                     │              │
                    YES            NO
                     │              │
                     ▼              ▼
              ┌────────────┐  ┌────────────────────┐
              │  Use QUIC  │  │ Mobile/Lossy net?  │
              └────────────┘  └────────────────────┘
                                  │          │
                                 YES        NO
                                  │          │
                                  ▼          ▼
                           ┌────────────┐ ┌────────────────┐
                           │  Use QUIC  │ │  HTTP/2 is     │
                           │            │ │  fine (or /3   │
                           └────────────┘ │  for future-   │
                                          │  proofing)     │
                                          └────────────────┘
```

---

## Key Takeaways

1. **QUIC = UDP + TLS 1.3 + Multiplexing** — All baked into one protocol
2. **Faster connections** — 1-RTT vs 3-RTT, or even 0-RTT for repeat visits
3. **No head-of-line blocking** — Lost packets only affect their own stream
4. **Works on bad networks** — Mobile, WiFi, high-latency all benefit
5. **Connection migration** — Seamlessly switch networks without dropping connections
6. **Always encrypted** — Security is mandatory, not optional
