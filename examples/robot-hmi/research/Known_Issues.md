# Known Issues - Comprehensive List

**CRITICAL**: These issues were identified by the user and must ALL be resolved. Do not consider the project complete until every issue is fixed.

---

## Issue #1: Number Inputs Must Be Text Inputs

**Status:** UNRESOLVED  
**Priority:** HIGH  
**Scope:** Application-wide

Number type `<input type="number">` inputs are extremely difficult to use with decimals and negative values. ALL number inputs must be replaced with text inputs (`<input type="text">`) with proper validation.

**Action Required:**
- Search entire codebase for `type="number"` or `type={"number"}`
- Replace with text inputs
- Add validation for numeric values
- This applies to speed, step, position values, etc.

---

## Issue #2: I/O Status Panel - Not Exact Replica

**Status:** UNRESOLVED  
**Priority:** HIGH

The I/O panel must match the original exactly:
- Ability to SET output types (DOUT, AOUT, GOUT)
- Properly READ and display input types (DIN, AIN, GIN)
- Highlighting when appropriate (active/triggered states)
- Exact visual match to original

**Reference:** `/home/apino/dev/Fanuc_RMI_API/web_app/src/components/layout/workspace/dashboard/control/io_panel.rs`

---

## Issue #3: Program Loading Completely Broken

**Status:** UNRESOLVED  
**Priority:** CRITICAL

When loading a program:
- Program list shows nothing
- Should show entire program content
- Should show progress bar
- Should show current line number
- Must implement ACTUAL robot program execution

**Reference:** Study original program execution flow extensively

---

## Issue #4: Console Logging Incomplete

**Status:** UNRESOLVED  
**Priority:** HIGH

The console must log all message types like the original:
- Research original for ALL message types
- Implement proper error messages
- Implement proper status messages
- Improve formatting but don't miss any types

**Reference:** `/home/apino/dev/Fanuc_RMI_API/src/api/messages.rs` for message types

---

## Issue #5: Command Composer Issues

**Status:** UNRESOLVED  
**Priority:** MEDIUM

Problems:
- Commands are created but NOT sent to robot
- "Add to Recent" should be "Apply"
- "Apply" should add to recent AND make it current selection
- Need ability to EDIT recent commands

---

## Issue #6: Configuration Tab - Multiple Issues

**Status:** UNRESOLVED  
**Priority:** HIGH

Problems:
- Default configuration not loading properly
- Must work EXACTLY as original with NO exceptions
- Active utool/uframe accordion should auto-expand
- Note nuanced accordion interaction behavior
- Individual and "all" expand/collapse buttons (top right)
- Backend logic and frontend communication must be complete

**Reference:** Study original configuration panel extensively

---

## Issue #7: Sidebar Missing Information

**Status:** UNRESOLVED  
**Priority:** MEDIUM

Missing from sidebar:
- Active Uframe display
- Active Utool display
- All other information the original has

Note: Joint angles were ADDED (not in original) - this can stay. But nothing from original should be MISSING.

---

## Issue #8: Pop-Out Functionality Missing

**Status:** UNRESOLVED  
**Priority:** MEDIUM

Both panels need pop-out capability:
- Jog Controls - pop out to appropriately sized widget
- I/O Controls - pop out to appropriately sized widget
- Must match original implementation exactly

---

## Issue #9: Jog Controls Missing W/P/R

**Status:** PARTIALLY RESOLVED  
**Priority:** MEDIUM

Original jog controls included:
- W/P/R rotation jog functionality
- Appropriate display below jog controls

Note: Some W/P/R was added but verify it matches original exactly.

---

## Issue #10: Jog Defaults - Server Overriding Input

**Status:** UNRESOLVED  
**Priority:** HIGH

On Configuration tab:
- Cannot enter different step/speed values
- Server's value immediately overrides user input
- Study how original solved this problem
- Implement exact same solution

---

## Issue #11: Program Opening/Creating/Editing Broken

**Status:** UNRESOLVED  
**Priority:** CRITICAL

Completely non-functional:
- Cannot open program to view/edit
- Creating program only adds to database
- Nothing displays about the program
- No content shown after creation
- Cannot upload anything

This is a MASSIVE problem requiring immediate attention.

---

## Issue #12: Robot Connection Editing Broken

**Status:** UNRESOLVED  
**Priority:** CRITICAL

Multiple problems:
- "Arm Configuration" values are number inputs instead of DROPDOWNS
- These are booleans (0 or 1 only) - use same components as wizard
- "Update" button causes infinite spinner (possible infinite loop)
- "Save Changes" does optimistic update but NOT database update
- Values revert when leaving and returning

---

## Issue #13: Quick Command Buttons Non-Functional

**Status:** UNRESOLVED  
**Priority:** HIGH

Abort, Reset, Initialize buttons:
- Do nothing when clicked
- No toast notification feedback
- No pending/loading state
- Original sent data to robot for TP program control
- Understand nuances of how original implemented this

---

## Issue #14: Joint Jogging Non-Functional

**Status:** UNRESOLVED  
**Priority:** HIGH

Problems:
- Does not send any data to robot
- Step/Speed values not loaded from robot defaults on connection
- Note: These defaults are DIFFERENT from "Default" in Configuration tab

---

## Issue #15: Control System Incomplete

**Status:** UNRESOLVED  
**Priority:** CRITICAL

Problems:
- No indication when ANOTHER client has control
- Only shows "control requested" then nothing
- Client disconnect does NOT release control
- May be bug in ExclusiveControlPlugin
- This is CATASTROPHIC for industrial applications

---

## Issue #16: Toast Notification Position Wrong

**Status:** UNRESOLVED  
**Priority:** LOW

Toast notifications are in wrong location. Match original's positioning exactly.

**Reference:** Check original's toast implementation

