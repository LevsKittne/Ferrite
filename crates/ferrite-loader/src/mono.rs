use std::ffi::{CStr, CString, c_char, c_int, c_void};
use crate::sys::{self, Module};

pub type Ptr = *mut c_void;

const TABLE_TYPEDEF: c_int = 2;
const TOKEN_TYPEDEF: u32 = 0x0200_0000;
const TYPE_ATTRIBUTE_ABSTRACT: u32 = 0x80;
const TYPE_ATTRIBUTE_INTERFACE: u32 = 0x20;

macro_rules! api {
    ($($field:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        #[allow(clippy::struct_field_names)]
        #[derive(Debug)]
        pub struct Mono {
            $($field: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Mono {
            pub fn load(module: Module) -> Result<Self, &'static str> {
                Ok(Self {
                    $($field: {
                        let name = concat!("mono_", stringify!($field), "\0");
                        let symbol = CStr::from_bytes_with_nul(name.as_bytes()).map_err(|_| name)?;
                        let f = sys::symbol(module, symbol).ok_or(name)?;
                        unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn($($arg),*) $(-> $ret)?>(f) }
                    },)*
                })
            }
        }
    };
}

api! {
    domain_get: fn() -> Ptr;
    domain_assembly_open: fn(Ptr, *const c_char) -> Ptr;
    assembly_get_image: fn(Ptr) -> Ptr;
    image_get_table_rows: fn(Ptr, c_int) -> c_int;
    class_get: fn(Ptr, u32) -> Ptr;
    class_from_name: fn(Ptr, *const c_char, *const c_char) -> Ptr;
    class_get_name: fn(Ptr) -> *const c_char;
    class_get_namespace: fn(Ptr) -> *const c_char;
    class_get_flags: fn(Ptr) -> u32;
    class_is_subclass_of: fn(Ptr, Ptr, c_int) -> c_int;
    class_get_type: fn(Ptr) -> Ptr;
    class_get_method_from_name: fn(Ptr, *const c_char, c_int) -> Ptr;
    class_get_field_from_name: fn(Ptr, *const c_char) -> Ptr;
    field_get_value: fn(Ptr, Ptr, *mut c_void);
    type_get_object: fn(Ptr, Ptr) -> Ptr;
    custom_attrs_from_class: fn(Ptr) -> Ptr;
    custom_attrs_get_attr: fn(Ptr, Ptr) -> Ptr;
    custom_attrs_free: fn(Ptr);
    runtime_invoke: fn(Ptr, Ptr, *mut Ptr, *mut Ptr) -> Ptr;
    object_to_string: fn(Ptr, *mut Ptr) -> Ptr;
    string_new_utf16: fn(Ptr, *const u16, c_int) -> Ptr;
    string_chars: fn(Ptr) -> *const u16;
    string_length: fn(Ptr) -> c_int;
    array_new: fn(Ptr, Ptr, usize) -> Ptr;
    array_length: fn(Ptr) -> usize;
    array_addr_with_size: fn(Ptr, c_int, usize) -> *mut Ptr;
    gc_wbarrier_set_arrayref: fn(Ptr, *mut Ptr, Ptr);
    get_corlib: fn() -> Ptr;
    add_internal_call: fn(*const c_char, *const c_void);
    thread_current: fn() -> Ptr;
    thread_set_main: fn(Ptr);
    image_open_from_data_with_name: fn(*mut c_char, u32, c_int, *mut c_int, c_int, *const c_char) -> Ptr;
    assembly_load_from_full: fn(Ptr, *const c_char, *mut c_int, c_int) -> Ptr;
    image_addref: fn(Ptr);
}

#[derive(Debug, Clone, Copy)]
pub struct Class(pub Ptr);

impl Mono {
    pub fn add_icall(&self, name: &str, f: *const c_void) {
        if let Ok(name) = CString::new(name) {
            unsafe { (self.add_internal_call)(name.as_ptr(), f) };
        }
    }

    pub fn assembly_image(&self, assembly: Ptr) -> Ptr {
        unsafe { (self.assembly_get_image)(assembly) }
    }

    pub fn image_addref(&self, image: Ptr) {
        unsafe { (self.image_addref)(image) };
    }

    pub unsafe fn bytes<'a>(&self, array: Ptr) -> &'a [u8] {
        if array.is_null() {
            return &[];
        }
        let len = unsafe { (self.array_length)(array) };
        let data = unsafe { (self.array_addr_with_size)(array, 1, 0) }.cast::<u8>();
        unsafe { std::slice::from_raw_parts(data, len) }
    }

    pub fn load_from_memory(&self, path: &str, data: &[u8]) -> Result<(Ptr, Ptr), String> {
        let name = CString::new(path).map_err(|e| e.to_string())?;
        let len = u32::try_from(data.len()).map_err(|e| e.to_string())?;
        let mut status: c_int = 0;
        let image = unsafe {
            (self.image_open_from_data_with_name)(
                data.as_ptr().cast_mut().cast(),
                len,
                1,
                &raw mut status,
                0,
                name.as_ptr(),
            )
        };
        if image.is_null() {
            return Err(format!("image open failed with status {status}"));
        }
        let assembly =
            unsafe { (self.assembly_load_from_full)(image, name.as_ptr(), &raw mut status, 0) };
        if assembly.is_null() {
            return Err(format!("assembly load failed with status {status}"));
        }
        Ok((assembly, image))
    }

    pub fn set_main_thread(&self) {
        unsafe { (self.thread_set_main)((self.thread_current)()) };
    }

    pub unsafe fn read_string(&self, s: Ptr) -> Option<String> {
        if s.is_null() {
            return None;
        }
        let len = usize::try_from(unsafe { (self.string_length)(s) }).unwrap_or_default();
        let chars = unsafe { (self.string_chars)(s) };
        Some(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(chars, len)
        }))
    }

    pub fn new_string(&self, s: &str) -> Ptr {
        let units: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            (self.string_new_utf16)(
                (self.domain_get)(),
                units.as_ptr(),
                c_int::try_from(units.len()).unwrap_or(c_int::MAX),
            )
        }
    }

    pub fn open_image(&self, path: &str) -> Option<Ptr> {
        let path = CString::new(path).ok()?;
        let asm = unsafe { (self.domain_assembly_open)((self.domain_get)(), path.as_ptr()) };
        if asm.is_null() {
            return None;
        }
        let image = unsafe { (self.assembly_get_image)(asm) };
        (!image.is_null()).then_some(image)
    }

    pub fn class(&self, image: Ptr, namespace: &str, name: &str) -> Option<Class> {
        let (ns, n) = (CString::new(namespace).ok()?, CString::new(name).ok()?);
        let class = unsafe { (self.class_from_name)(image, ns.as_ptr(), n.as_ptr()) };
        (!class.is_null()).then_some(Class(class))
    }

    pub fn classes(&self, image: Ptr) -> impl Iterator<Item = Class> + '_ {
        let rows = u32::try_from(unsafe { (self.image_get_table_rows)(image, TABLE_TYPEDEF) })
            .unwrap_or_default();
        (2..=rows).filter_map(move |row| {
            let class = unsafe { (self.class_get)(image, TOKEN_TYPEDEF | row) };
            (!class.is_null()).then_some(Class(class))
        })
    }

    pub fn full_name(&self, class: Class) -> String {
        let read = |p: *const c_char| {
            if p.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
            }
        };
        let ns = read(unsafe { (self.class_get_namespace)(class.0) });
        let name = read(unsafe { (self.class_get_name)(class.0) });
        if ns.is_empty() {
            name
        } else {
            format!("{ns}.{name}")
        }
    }

    pub fn is_concrete_subclass(&self, class: Class, base: Class) -> bool {
        let flags = unsafe { (self.class_get_flags)(class.0) };
        flags & (TYPE_ATTRIBUTE_ABSTRACT | TYPE_ATTRIBUTE_INTERFACE) == 0
            && unsafe { (self.class_is_subclass_of)(class.0, base.0, 0) } != 0
    }

    pub fn attribute(&self, class: Class, attr: Class) -> Option<Ptr> {
        let info = unsafe { (self.custom_attrs_from_class)(class.0) };
        if info.is_null() {
            return None;
        }
        let obj = unsafe { (self.custom_attrs_get_attr)(info, attr.0) };
        unsafe { (self.custom_attrs_free)(info) };
        (!obj.is_null()).then_some(obj)
    }

    pub fn object_field(&self, obj: Ptr, class: Class, name: &str) -> Ptr {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null_mut();
        };
        let field = unsafe { (self.class_get_field_from_name)(class.0, name.as_ptr()) };
        if field.is_null() {
            return std::ptr::null_mut();
        }
        let mut value: Ptr = std::ptr::null_mut();
        unsafe { (self.field_get_value)(obj, field, (&raw mut value).cast()) };
        value
    }

    pub fn string_field(&self, obj: Ptr, class: Class, name: &str) -> Option<String> {
        unsafe { self.read_string(self.object_field(obj, class, name)) }
    }

    pub fn string_array_field(&self, obj: Ptr, class: Class, name: &str) -> Vec<String> {
        let array = self.object_field(obj, class, name);
        if array.is_null() {
            return Vec::new();
        }
        let len = unsafe { (self.array_length)(array) };
        let size = c_int::try_from(size_of::<Ptr>()).unwrap_or(8);
        (0..len)
            .filter_map(|i| unsafe {
                self.read_string(*(self.array_addr_with_size)(array, size, i))
            })
            .collect()
    }

    pub fn type_array(&self, classes: &[Class]) -> Ptr {
        let domain = unsafe { (self.domain_get)() };
        let Some(type_class) = self.class(unsafe { (self.get_corlib)() }, "System", "Type") else {
            return std::ptr::null_mut();
        };
        let array = unsafe { (self.array_new)(domain, type_class.0, classes.len()) };
        let size = c_int::try_from(size_of::<Ptr>()).unwrap_or(8);
        for (i, class) in classes.iter().enumerate() {
            let object = unsafe { (self.type_get_object)(domain, (self.class_get_type)(class.0)) };
            unsafe {
                (self.gc_wbarrier_set_arrayref)(
                    array,
                    (self.array_addr_with_size)(array, size, i),
                    object,
                );
            };
        }
        array
    }

    pub fn invoke_static(
        &self,
        image: Ptr,
        namespace: &str,
        class: &str,
        method: &str,
    ) -> Result<(), String> {
        let klass = self
            .class(image, namespace, class)
            .ok_or_else(|| format!("class {namespace}.{class} not found"))?;
        let name = CString::new(method).map_err(|e| e.to_string())?;
        let m = unsafe { (self.class_get_method_from_name)(klass.0, name.as_ptr(), 0) };
        if m.is_null() {
            return Err(format!("method {namespace}.{class}.{method}() not found"));
        }
        let mut exc: Ptr = std::ptr::null_mut();
        unsafe {
            (self.runtime_invoke)(m, std::ptr::null_mut(), std::ptr::null_mut(), &raw mut exc)
        };
        if exc.is_null() {
            Ok(())
        } else {
            Err(self.describe(exc))
        }
    }

    fn describe(&self, obj: Ptr) -> String {
        let mut exc: Ptr = std::ptr::null_mut();
        let s = unsafe { (self.object_to_string)(obj, &raw mut exc) };
        if exc.is_null() {
            unsafe { self.read_string(s) }.unwrap_or_else(|| "managed exception".into())
        } else {
            "managed exception (ToString failed)".into()
        }
    }
}
