//! Lightweight accessibility taxonomy (logical roles + names).
//! Native platform adapters still need to expose these to OS APIs (UIAutomation / NSAccessibility / AT-SPI).

/// High-level accessibility role analogous to WinForms [`Control.AccessibleRole`](https://learn.microsoft.com/dotnet/api/system.windows.forms.accessiblerole).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AccessibleRole {
    Window,
    Button,
    TextField,
    Label,
    Pane,
    Dialog,
}

/// Declarative description that a future IA2/ATS bridge can ingest.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AccessibleAnnouncement {
    pub role: AccessibleRole,
    pub name: Option<String>,
    pub description: Option<String>,
}
