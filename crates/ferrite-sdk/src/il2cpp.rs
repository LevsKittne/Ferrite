use crate::Context;
use std::ffi::{CString, c_char, c_void};

pub type Ptr = *mut c_void;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class(pub Ptr);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Method(pub Ptr);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Object(pub Ptr);

unsafe impl Send for Class {}
unsafe impl Send for Method {}

impl Method {
    #[must_use]
    pub fn pointer(self) -> Ptr {
        unsafe { *self.0.cast::<Ptr>() }
    }

    pub unsafe fn replace_pointer(self, replacement: Ptr) -> Ptr {
        let slots = self.0.cast::<Ptr>();
        let previous = unsafe { slots.read() };
        unsafe { slots.write(replacement) };
        if unsafe { slots.add(1).read() } == previous {
            unsafe { slots.add(1).write(replacement) };
        }
        previous
    }
}

impl Object {
    #[must_use]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

macro_rules! api {
    ($($field:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        #[allow(clippy::struct_field_names)]
        #[derive(Debug, Clone, Copy)]
        pub struct Il2Cpp {
            $($field: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }

        unsafe impl Send for Il2Cpp {}
        unsafe impl Sync for Il2Cpp {}

        impl Il2Cpp {
            pub fn from_resolver(resolve: impl Fn(&str) -> Option<Ptr>) -> Option<Self> {
                Some(Self {
                    $($field: {
                        let p = resolve(concat!("il2cpp_", stringify!($field)))?;
                        unsafe { std::mem::transmute::<Ptr, unsafe extern "C" fn($($arg),*) $(-> $ret)?>(p) }
                    },)*
                })
            }
        }
    };
}

api! {
    domain_get: fn() -> Ptr;
    domain_get_assemblies: fn(Ptr, *mut usize) -> *const Ptr;
    assembly_get_image: fn(Ptr) -> Ptr;
    class_from_name: fn(Ptr, *const c_char, *const c_char) -> Ptr;
    class_get_method_from_name: fn(Ptr, *const c_char, i32) -> Ptr;
    class_get_field_from_name: fn(Ptr, *const c_char) -> Ptr;
    field_get_offset: fn(Ptr) -> usize;
    field_static_get_value: fn(Ptr, *mut c_void);
    runtime_invoke: fn(Ptr, Ptr, *mut Ptr, *mut Ptr) -> Ptr;
    object_unbox: fn(Ptr) -> Ptr;
    string_new: fn(*const c_char) -> Ptr;
    string_chars: fn(Ptr) -> *const u16;
    string_length: fn(Ptr) -> i32;
    thread_attach: fn(Ptr) -> Ptr;
}

impl Il2Cpp {
    #[must_use]
    pub fn load(ctx: &Context) -> Option<Self> {
        Self::from_resolver(|name| ctx.symbol(name))
    }

    pub fn attach_thread(&self) {
        unsafe { (self.thread_attach)((self.domain_get)()) };
    }

    #[must_use]
    pub fn class(&self, namespace: &str, name: &str) -> Option<Class> {
        let (ns, n) = (CString::new(namespace).ok()?, CString::new(name).ok()?);
        let mut count = 0usize;
        let assemblies =
            unsafe { (self.domain_get_assemblies)((self.domain_get)(), &raw mut count) };
        if assemblies.is_null() {
            return None;
        }
        (0..count).find_map(|i| {
            let image = unsafe { (self.assembly_get_image)(*assemblies.add(i)) };
            let class = unsafe { (self.class_from_name)(image, ns.as_ptr(), n.as_ptr()) };
            (!class.is_null()).then_some(Class(class))
        })
    }

    #[must_use]
    pub fn method(&self, class: Class, name: &str, args: i32) -> Option<Method> {
        let name = CString::new(name).ok()?;
        let method = unsafe { (self.class_get_method_from_name)(class.0, name.as_ptr(), args) };
        (!method.is_null()).then_some(Method(method))
    }

    #[must_use]
    pub fn find_method(
        &self,
        namespace: &str,
        class: &str,
        name: &str,
        args: i32,
    ) -> Option<Method> {
        self.method(self.class(namespace, class)?, name, args)
    }

    #[must_use]
    pub fn field_offset(&self, class: Class, name: &str) -> Option<usize> {
        let name = CString::new(name).ok()?;
        let field = unsafe { (self.class_get_field_from_name)(class.0, name.as_ptr()) };
        (!field.is_null()).then(|| unsafe { (self.field_get_offset)(field) })
    }

    #[must_use]
    pub unsafe fn static_field<T: Copy + Default>(&self, class: Class, name: &str) -> Option<T> {
        let name = CString::new(name).ok()?;
        let field = unsafe { (self.class_get_field_from_name)(class.0, name.as_ptr()) };
        if field.is_null() {
            return None;
        }
        let mut value = T::default();
        unsafe { (self.field_static_get_value)(field, (&raw mut value).cast()) };
        Some(value)
    }

    pub unsafe fn invoke(
        &self,
        method: Method,
        instance: Object,
        args: &mut [Ptr],
    ) -> Result<Object, Object> {
        let mut exception: Ptr = std::ptr::null_mut();
        let params = if args.is_empty() {
            std::ptr::null_mut()
        } else {
            args.as_mut_ptr()
        };
        let result =
            unsafe { (self.runtime_invoke)(method.0, instance.0, params, &raw mut exception) };
        if exception.is_null() {
            Ok(Object(result))
        } else {
            Err(Object(exception))
        }
    }

    #[must_use]
    pub unsafe fn unbox<T: Copy>(&self, boxed: Object) -> Option<T> {
        if boxed.is_null() {
            return None;
        }
        Some(unsafe { *(self.object_unbox)(boxed.0).cast::<T>() })
    }

    #[must_use]
    pub fn string(&self, value: &str) -> Object {
        CString::new(value).map_or(Object(std::ptr::null_mut()), |c| {
            Object(unsafe { (self.string_new)(c.as_ptr()) })
        })
    }

    #[must_use]
    pub unsafe fn read_string(&self, value: Object) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let len = usize::try_from(unsafe { (self.string_length)(value.0) }).ok()?;
        let chars = unsafe { (self.string_chars)(value.0) };
        Some(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(chars, len)
        }))
    }
}
