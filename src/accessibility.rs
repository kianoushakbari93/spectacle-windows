//! UI Automation support for the Settings window, so that screen readers
//! such as Narrator can read and operate its controls.
//!
//! The window is a fragment root whose children are its controls: shortcut
//! recorders and the run mode picker are buttons with a value, the sign-in
//! option is a check box. Providers only identify a control by its position
//! in the tab order and ask the Settings window for everything else, so they
//! never hold on to its state.

// The UI Automation identifiers keep their Windows SDK names, which are not
// upper case, and are matched on below.
#![allow(non_upper_case_globals)]

use std::mem::ManuallyDrop;
use std::sync::{Mutex, mpsc};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{SafeArrayCreateVector, SafeArrayPutElement};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::System::Variant::{
    VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_BSTR, VT_EMPTY, VT_I4,
};
use windows::Win32::UI::Accessibility::{
    IInvokeProvider, IInvokeProvider_Impl, IRawElementProviderFragment,
    IRawElementProviderFragment_Impl, IRawElementProviderFragmentRoot,
    IRawElementProviderFragmentRoot_Impl, IRawElementProviderSimple,
    IRawElementProviderSimple_Impl, IToggleProvider, IToggleProvider_Impl, IValueProvider,
    IValueProvider_Impl, NavigateDirection, NavigateDirection_FirstChild,
    NavigateDirection_LastChild, NavigateDirection_NextSibling, NavigateDirection_Parent,
    NavigateDirection_PreviousSibling, ProviderOptions, ProviderOptions_ServerSideProvider,
    ToggleState, ToggleState_Off, ToggleState_On, UIA_AutomationFocusChangedEventId,
    UIA_AutomationIdPropertyId, UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId,
    UIA_ControlTypePropertyId, UIA_HasKeyboardFocusPropertyId, UIA_HelpTextPropertyId,
    UIA_InvokePatternId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId,
    UIA_NamePropertyId, UIA_PATTERN_ID, UIA_PROPERTY_ID, UIA_TextControlTypeId,
    UIA_TogglePatternId, UIA_ValuePatternId, UiaAppendRuntimeId, UiaClientsAreListening,
    UiaHostProviderFromHwnd, UiaRaiseAutomationEvent, UiaRect, UiaReturnRawElementProvider,
    UiaRootObjectId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowThreadProcessId, PostMessageW, SendMessageW,
};
use windows::core::{BOOL, BSTR, Error, IUnknown, Interface, PCWSTR, Result, implement};
use windows_core::IUnknownImpl;

use crate::dialogs;
use crate::preferences::{self, AccessibleElement, AccessibleKind};

/// Posted to the Settings window to activate a control asked for by an
/// assistive technology, outside of the UI Automation call.
pub const ACTIVATE_MESSAGE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 10;

/// Sent to the Settings window to run a pending call on its thread.
pub const CALL_MESSAGE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 11;

/// A call waiting to run on the Settings window's thread. The message that
/// runs it carries no data, so no other process can make Spectacle run
/// anything but the pending call.
type PendingCall = Box<dyn FnOnce() + Send>;
static PENDING_CALL: Mutex<Option<PendingCall>> = Mutex::new(None);
/// Lets one thread at a time use the pending call slot.
static CALL_LOCK: Mutex<()> = Mutex::new(());

/// UI Automation calls providers on its own threads, while the Settings
/// window's state belongs to the window's thread; this runs `call` there.
fn on_window_thread<R: Send + 'static>(
    hwnd: HWND,
    call: impl FnOnce() -> R + Send + 'static,
) -> Option<R> {
    // SAFETY: plain thread queries.
    let window_thread = unsafe { GetWindowThreadProcessId(hwnd, None) };
    if window_thread == 0 {
        return None;
    }
    // SAFETY: as above.
    if window_thread == unsafe { GetCurrentThreadId() } {
        return Some(call());
    }
    let _serialized = CALL_LOCK.lock().ok()?;
    let (sender, receiver) = mpsc::channel();
    *PENDING_CALL.lock().ok()? = Some(Box::new(move || {
        let _ = sender.send(call());
    }));
    // SAFETY: a synchronous message to the window, which runs the call.
    unsafe { SendMessageW(hwnd, CALL_MESSAGE, None, None) };
    // Take the call back if the window was gone and never ran it.
    PENDING_CALL.lock().ok()?.take();
    receiver.try_recv().ok()
}

/// Runs the pending call; the Settings window does this for CALL_MESSAGE.
pub fn run_pending_call() {
    let call = PENDING_CALL
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    if let Some(call) = call {
        call();
    }
}

/// The windows that provide controls: the Settings window and dialogs.
#[derive(Clone, Copy)]
enum Source {
    Settings,
    Dialog(isize),
}

impl Source {
    fn of(hwnd: HWND) -> Source {
        let mut class = [0u16; 64];
        // SAFETY: the buffer length is passed implicitly by the slice.
        let length = unsafe { GetClassNameW(hwnd, &mut class) };
        // SAFETY: CLASS_NAME is a valid null-terminated string.
        let dialog = unsafe { dialogs::CLASS_NAME.as_wide() };
        if usize::try_from(length).is_ok_and(|length| &class[..length] == dialog) {
            Source::Dialog(hwnd.0 as isize)
        } else {
            Source::Settings
        }
    }

    fn dialog(hwnd: isize) -> HWND {
        HWND(hwnd as *mut _)
    }

    fn count(self) -> usize {
        match self {
            Source::Settings => preferences::accessible_count(),
            Source::Dialog(hwnd) => dialogs::accessible_count(Self::dialog(hwnd)),
        }
    }

    fn element(self, index: usize) -> Option<AccessibleElement> {
        match self {
            Source::Settings => preferences::accessible_element(index),
            Source::Dialog(hwnd) => dialogs::accessible_element(Self::dialog(hwnd), index),
        }
    }

    fn hit_test(self, x: f32, y: f32) -> Option<usize> {
        match self {
            Source::Settings => preferences::accessible_hit_test(x, y),
            Source::Dialog(hwnd) => dialogs::accessible_hit_test(Self::dialog(hwnd), x, y),
        }
    }

    fn focus(self) -> Option<usize> {
        match self {
            Source::Settings => preferences::accessible_focus(),
            Source::Dialog(hwnd) => dialogs::accessible_focus(Self::dialog(hwnd)),
        }
    }

    fn set_focus(self, index: usize) {
        match self {
            Source::Settings => preferences::accessible_set_focus(index),
            Source::Dialog(hwnd) => dialogs::accessible_set_focus(Self::dialog(hwnd), index),
        }
    }
}

fn control_count(hwnd: HWND) -> usize {
    let source = Source::of(hwnd);
    on_window_thread(hwnd, move || source.count()).unwrap_or(0)
}

fn control_element(hwnd: HWND, index: usize) -> Option<AccessibleElement> {
    let source = Source::of(hwnd);
    on_window_thread(hwnd, move || source.element(index)).flatten()
}

/// UIA_E_ELEMENTNOTAVAILABLE: the control is gone or busy.
fn element_not_available() -> Error {
    Error::from_hresult(windows::core::HRESULT(0x8004_0201_u32 as i32))
}

/// Answers WM_GETOBJECT for the Settings window.
pub fn handle_get_object(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if lparam.0 as i32 != UiaRootObjectId {
        return None;
    }
    let root: IRawElementProviderSimple = Root { hwnd }.into();
    // SAFETY: hands the provider to UI Automation for this window.
    Some(unsafe { UiaReturnRawElementProvider(hwnd, wparam, lparam, &root) })
}

/// Releases the providers handed out for a window that is being destroyed.
pub fn window_destroyed(hwnd: HWND) {
    // SAFETY: documented way to disconnect a window's providers.
    unsafe { UiaReturnRawElementProvider(hwnd, WPARAM(0), LPARAM(0), None) };
}

/// Tells assistive technologies which control has the keyboard focus.
pub fn focus_changed(hwnd: HWND, index: usize) {
    // SAFETY: plain queries and an event on this thread's window.
    unsafe {
        if !UiaClientsAreListening().as_bool() {
            return;
        }
        let provider: IRawElementProviderSimple = Control { hwnd, index }.into();
        let _ = UiaRaiseAutomationEvent(&provider, UIA_AutomationFocusChangedEventId);
    }
}

fn variant_i32(value: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: value },
            }),
        },
    }
}

fn variant_bool(value: bool) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_BOOL,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 {
                    boolVal: windows::Win32::Foundation::VARIANT_BOOL(if value { -1 } else { 0 }),
                },
            }),
        },
    }
}

fn variant_string(value: &str) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_BSTR,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 {
                    bstrVal: ManuallyDrop::new(BSTR::from(value)),
                },
            }),
        },
    }
}

fn variant_empty() -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_EMPTY,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { llVal: 0 },
            }),
        },
    }
}

/// The Settings window as a whole.
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IRawElementProviderFragmentRoot
)]
struct Root {
    hwnd: HWND,
}

/// One control of the Settings window, by its position in the tab order.
#[implement(
    IRawElementProviderSimple,
    IRawElementProviderFragment,
    IInvokeProvider,
    IToggleProvider,
    IValueProvider
)]
struct Control {
    hwnd: HWND,
    index: usize,
}

impl IRawElementProviderSimple_Impl for Root_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, _: UIA_PATTERN_ID) -> Result<IUnknown> {
        Err(Error::empty())
    }

    fn GetPropertyValue(&self, _: UIA_PROPERTY_ID) -> Result<VARIANT> {
        // The window's own properties come from its host provider.
        Ok(variant_empty())
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        // SAFETY: the window belongs to this thread.
        unsafe { UiaHostProviderFromHwnd(self.hwnd) }
    }
}

impl IRawElementProviderFragment_Impl for Root_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        let count = control_count(self.hwnd);
        let index = match direction {
            NavigateDirection_FirstChild if count > 0 => 0,
            NavigateDirection_LastChild if count > 0 => count - 1,
            _ => return Err(Error::empty()),
        };
        Ok(Control {
            hwnd: self.hwnd,
            index,
        }
        .into())
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        // The host provider supplies the window's runtime identifier.
        Ok(std::ptr::null_mut())
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        // The host provider supplies the window's bounds.
        Ok(UiaRect::default())
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> Result<()> {
        Ok(())
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        Ok(Root { hwnd: self.hwnd }.into())
    }
}

impl IRawElementProviderFragmentRoot_Impl for Root_Impl {
    fn ElementProviderFromPoint(&self, x: f64, y: f64) -> Result<IRawElementProviderFragment> {
        let mut point = POINT {
            x: x.round() as i32,
            y: y.round() as i32,
        };
        // SAFETY: converts coordinates for this thread's window.
        let _ = unsafe { ScreenToClient(self.hwnd, &mut point) };
        let (x, y) = (point.x as f32, point.y as f32);
        let source = Source::of(self.hwnd);
        match on_window_thread(self.hwnd, move || source.hit_test(x, y)).flatten() {
            Some(index) => Ok(Control {
                hwnd: self.hwnd,
                index,
            }
            .into()),
            None => Ok(Root { hwnd: self.hwnd }.into()),
        }
    }

    fn GetFocus(&self) -> Result<IRawElementProviderFragment> {
        let source = Source::of(self.hwnd);
        match on_window_thread(self.hwnd, move || source.focus()).flatten() {
            Some(index) => Ok(Control {
                hwnd: self.hwnd,
                index,
            }
            .into()),
            None => Err(Error::empty()),
        }
    }
}

impl Control_Impl {
    fn element(&self) -> Result<AccessibleElement> {
        control_element(self.hwnd, self.index).ok_or_else(element_not_available)
    }

    fn activate(&self) -> Result<()> {
        // Activation can open dialogs, which must not run inside a UI
        // Automation call.
        // SAFETY: posts to this thread's window.
        unsafe {
            PostMessageW(
                Some(self.hwnd),
                ACTIVATE_MESSAGE,
                WPARAM(self.index),
                LPARAM(0),
            )
        }
    }
}

impl IRawElementProviderSimple_Impl for Control_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, pattern: UIA_PATTERN_ID) -> Result<IUnknown> {
        let element = self.element()?;
        let supported = match element.kind {
            AccessibleKind::Recorder | AccessibleKind::RunMode => {
                pattern == UIA_InvokePatternId || pattern == UIA_ValuePatternId
            }
            AccessibleKind::CheckBox => pattern == UIA_TogglePatternId,
            AccessibleKind::Button => pattern == UIA_InvokePatternId,
            AccessibleKind::Text => false,
        };
        if !supported {
            return Err(Error::empty());
        }
        let this: IRawElementProviderSimple = self.to_interface();
        this.cast()
    }

    fn GetPropertyValue(&self, property: UIA_PROPERTY_ID) -> Result<VARIANT> {
        let element = self.element()?;
        Ok(match property {
            UIA_ControlTypePropertyId => variant_i32(match element.kind {
                AccessibleKind::CheckBox => UIA_CheckBoxControlTypeId.0,
                AccessibleKind::Text => UIA_TextControlTypeId.0,
                _ => UIA_ButtonControlTypeId.0,
            }),
            UIA_NamePropertyId => variant_string(&element.name),
            UIA_AutomationIdPropertyId => variant_string(&element.automation_id),
            UIA_HelpTextPropertyId => match &element.help {
                Some(help) => variant_string(help),
                None => variant_empty(),
            },
            UIA_IsKeyboardFocusablePropertyId => {
                variant_bool(!matches!(element.kind, AccessibleKind::Text))
            }
            UIA_IsEnabledPropertyId => variant_bool(true),
            UIA_HasKeyboardFocusPropertyId => variant_bool(element.focused),
            _ => variant_empty(),
        })
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        Err(Error::empty())
    }
}

impl IRawElementProviderFragment_Impl for Control_Impl {
    fn Navigate(&self, direction: NavigateDirection) -> Result<IRawElementProviderFragment> {
        let count = control_count(self.hwnd);
        let index = match direction {
            NavigateDirection_Parent => return Ok(Root { hwnd: self.hwnd }.into()),
            NavigateDirection_NextSibling if self.index + 1 < count => self.index + 1,
            NavigateDirection_PreviousSibling if self.index > 0 => self.index - 1,
            _ => return Err(Error::empty()),
        };
        Ok(Control {
            hwnd: self.hwnd,
            index,
        }
        .into())
    }

    fn GetRuntimeId(&self) -> Result<*mut SAFEARRAY> {
        let identifiers = [UiaAppendRuntimeId as i32, self.index as i32 + 1];
        // SAFETY: creates a two element array of 32-bit integers, which UI
        // Automation takes ownership of.
        unsafe {
            let array = SafeArrayCreateVector(VT_I4, 0, identifiers.len() as u32);
            if array.is_null() {
                return Err(Error::from_hresult(
                    windows::Win32::Foundation::E_OUTOFMEMORY,
                ));
            }
            for (position, identifier) in identifiers.iter().enumerate() {
                let position = position as i32;
                SafeArrayPutElement(array, &position, (identifier as *const i32).cast())?;
            }
            Ok(array)
        }
    }

    fn BoundingRectangle(&self) -> Result<UiaRect> {
        let bounds = self.element()?.screen_bounds;
        Ok(UiaRect {
            left: f64::from(bounds.left),
            top: f64::from(bounds.top),
            width: f64::from(bounds.right - bounds.left),
            height: f64::from(bounds.bottom - bounds.top),
        })
    }

    fn GetEmbeddedFragmentRoots(&self) -> Result<*mut SAFEARRAY> {
        Ok(std::ptr::null_mut())
    }

    fn SetFocus(&self) -> Result<()> {
        let index = self.index;
        let source = Source::of(self.hwnd);
        on_window_thread(self.hwnd, move || source.set_focus(index));
        Ok(())
    }

    fn FragmentRoot(&self) -> Result<IRawElementProviderFragmentRoot> {
        Ok(Root { hwnd: self.hwnd }.into())
    }
}

impl IInvokeProvider_Impl for Control_Impl {
    fn Invoke(&self) -> Result<()> {
        self.element()?;
        self.activate()
    }
}

impl IToggleProvider_Impl for Control_Impl {
    fn Toggle(&self) -> Result<()> {
        self.element()?;
        self.activate()
    }

    fn ToggleState(&self) -> Result<ToggleState> {
        Ok(if self.element()?.toggled == Some(true) {
            ToggleState_On
        } else {
            ToggleState_Off
        })
    }
}

impl IValueProvider_Impl for Control_Impl {
    fn SetValue(&self, _: &PCWSTR) -> Result<()> {
        // Shortcuts are recorded from the keyboard, not typed in.
        Err(Error::from_hresult(
            windows::Win32::Foundation::E_ACCESSDENIED,
        ))
    }

    fn Value(&self) -> Result<BSTR> {
        Ok(BSTR::from(self.element()?.value.unwrap_or_default()))
    }

    fn IsReadOnly(&self) -> Result<BOOL> {
        Ok(BOOL::from(true))
    }
}
