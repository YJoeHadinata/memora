# Memora Architecture

## 1. Overview

Memora is organized with clear separation between kernel interfaces, business logic domain services, and immediate-mode GUI rendering.

```text
macOS Mach & Sysctl APIs (host_statistics64, sysctl, libproc)
                             │
                             ▼
                   System Layer (src/system/)
              ├── MemoryCollector (MacosMemoryCollector)
              └── ProcessCollector & ProcessManager (MacosProcessCollector)
                             │
                             ▼
                   Domain Models (src/models/)
              ├── MemorySnapshot & MemoryPressure
              ├── ProcessInfo
              ├── CacheItem & CacheScanResult
              └── HistoryBuffer & HistoryPoint
                             │
                             ▼
                  Services Layer (src/services/)
              ├── MemoryService
              ├── ProcessService
              ├── HistoryService
              └── CacheService
                             │
                             ▼
               Application State & Background Worker (src/app.rs)
                             │
                             ▼
                    User Interface (src/ui/)
              ├── Dashboard View (egui_plot + metrics)
              ├── Processes View (sortable table + actions)
              ├── Caches View (RAM purge + disk cache cleaning)
              └── Settings View
```

## 2. Concurrency & Performance Design

- **Non-blocking UI**: Process inspection across hundreds of system PIDs, Mach host statistics, and disk cache scanning are performed asynchronously in dedicated background worker threads.
- **Message Passing**: The collector thread and cache workers send `BackgroundMessage` across standard `std::sync::mpsc` channels.
- **Cache Management**: Provides transparent, non-deceptive memory management. Exposes inactive file cache in RAM (`external_page_count` + `purgeable_count`) with native `/usr/sbin/purge` execution, and safe user application disk cache inspection (`~/Library/Caches`).
- **Bounded Buffers**: Historical memory data points are stored in a pre-allocated ring buffer (`HistoryBuffer`) to prevent memory growth over extended monitoring sessions.
- **Offline & Private**: Zero external network connections, zero telemetry, zero analytics.
