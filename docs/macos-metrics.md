# macOS Memory & System Metrics Reference

This document explains the technical implementation and mathematical formulas used by **Memora** to collect and present memory metrics on macOS.

---

## 1. Metric Sources

Memora avoids parsing stdout of command-line tools such as `vm_stat` or `top`. Instead, it communicates directly with macOS XNU kernel interfaces:

1. **Mach Host Statistics**: `host_statistics64` with `HOST_VM_INFO64` returns `vm_statistics64_data_t`.
2. **Mach Page Size**: `host_page_size` dynamically determines page size (`16384` bytes on Apple Silicon).
3. **Hardware Memory Size**: `sysctlbyname("hw.memsize", ...)` returns total physical RAM in bytes.
4. **Swap Usage**: `sysctlbyname("vm.swapusage", ...)` returns `struct xsw_usage`.
5. **Memory Pressure Level**: `sysctlbyname("kern.memorystatus_vm_pressure_level", ...)` returns kernel memory pressure flags.
6. **Process Memory Footprint**: `proc_pid_rusage(pid, RUSAGE_INFO_V4, ...)` returns `ri_phys_footprint`, the canonical memory measurement used by Apple Activity Monitor.

---

## 2. Activity Monitor Formula Reconciliation

Modern macOS virtual memory uses compression and file caching heavily. A low "Free Memory" value does not mean the system is out of memory.

| Metric | Formula |
|---|---|
| **Page Size** | Dynamic via `host_page_size(mach_host_self(), &page_size)` |
| **App Memory** | `(internal_page_count - purgeable_count) * page_size` |
| **Wired Memory** | `wire_count * page_size` |
| **Compressed Memory** | `compressor_page_count * page_size` |
| **Used Memory** | `App Memory + Wired Memory + Compressed Memory` |
| **Cached Files** | `(purgeable_count + external_page_count) * page_size` |
| **Free Memory** | `free_count * page_size` |
| **Available Memory** | `Physical Memory - Used Memory` (Free + Reclaimable Cache) |

---

## 3. Memory Pressure Semantics

macOS manages memory through **pressure levels**, evaluated continuously by the kernel memory compressor and paging system:

- **Normal (Value 1 / `DISPATCH_MEMORYPRESSURE_NORMAL`)**: System has plenty of free and cached memory. Compression is minimal.
- **Warning (Value 2 / `DISPATCH_MEMORYPRESSURE_WARN`)**: System memory pressure is elevated. Memory compressor is actively compressing idle pages.
- **Critical (Value 4 / `DISPATCH_MEMORYPRESSURE_CRITICAL`)**: Severe memory pressure. Kernel is swapping compressed pages to disk (`vm.swapusage`) and sending notification signals to processes to release memory.

---

## 4. Process Memory: Resident vs Physical Footprint

Traditional UNIX tools report `RSS` (Resident Set Size). On macOS, RSS includes shared libraries and memory-mapped files shared across multiple processes, leading to significant overcounting.

Memora uses **`ri_phys_footprint`** from `proc_pid_rusage` flavor `RUSAGE_INFO_V4`. Physical footprint measures:
- Anonymous dirty memory allocated by the process.
- Pages compressed on behalf of the process in the compressor.
- IOKit and driver allocations tied to the process task.

This directly matches the "Memory" column in macOS Activity Monitor.

---

## 5. Protected Processes

To protect system integrity, Memora explicitly refuses Safe Quit and Force Quit actions for:
- PID 0 (`kernel_task`)
- PID 1 (`launchd`)
- Critical system daemons: `WindowServer`, `loginwindow`, `syspolicyd`, `opendirectoryd`, `securityd`, `diskarbitrationd`, `powerd`, `coreauthd`, `mds`, `notifyd`, `configd`.
