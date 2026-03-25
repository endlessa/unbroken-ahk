# unbroken-ahk

AutoHotkey scripts for automation.

## left-click-hold-loop.ahk

Toggles an auto-clicker that alternates between holding and releasing the left mouse button on a timed interval. Press **F1** to start/stop.

### How It Works

```mermaid
flowchart TD
    A[Press F1] --> B[ClickLoop called]
    B --> C[Send Q key down]
    B --> D[Toggle state ON/OFF]
    D -->|ON| E[Start timer every 2s]
    D -->|OFF| F[Stop timer]
    E --> G[ClickClick runs on interval]
```

### Click Toggle Cycle

```mermaid
stateDiagram-v2
    [*] --> MouseUp: Timer starts
    MouseUp --> MouseDown: Click Down
    MouseDown --> MouseUp: Click Up
    MouseUp --> [*]: F1 pressed (timer off)
    MouseDown --> [*]: F1 pressed (timer off)
```

### Execution Flow

```mermaid
sequenceDiagram
    participant User
    participant F1 as F1 Hotkey
    participant CL as ClickLoop()
    participant Timer as SetTimer
    participant CC as ClickClick Label

    User->>F1: Press F1
    F1->>CL: Call ClickLoop()
    CL->>CL: Send {q down}
    CL->>CL: Toggle state (ON/OFF)
    alt Toggler is ON
        CL->>Timer: Start every 2000ms
        loop Every 2 seconds
            Timer->>CC: Fire ClickClick
            alt mouseUp = true
                CC->>CC: Click Down
            else mouseUp = false
                CC->>CC: Click Up
            end
            CC->>CC: Flip mouseUp
        end
    else Toggler is OFF
        CL->>Timer: Stop timer
    end
    User->>F1: Press F1 again to stop
```
