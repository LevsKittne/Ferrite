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
    object_new: fn(Ptr) -> Ptr;
    object_get_class: fn(Ptr) -> Ptr;
    class_get_name: fn(Ptr) -> *const c_char;
    class_get_namespace: fn(Ptr) -> *const c_char;
    class_get_parent: fn(Ptr) -> Ptr;
    class_get_methods: fn(Ptr, *mut Ptr) -> Ptr;
    class_get_fields: fn(Ptr, *mut Ptr) -> Ptr;
    class_get_flags: fn(Ptr) -> i32;
    class_is_valuetype: fn(Ptr) -> bool;
    class_get_type: fn(Ptr) -> Ptr;
    method_get_name: fn(Ptr) -> *const c_char;
    method_get_param_count: fn(Ptr) -> u32;
    method_get_param: fn(Ptr, u32) -> Ptr;
    method_get_param_name: fn(Ptr, u32) -> *const c_char;
    method_get_return_type: fn(Ptr) -> Ptr;
    method_get_flags: fn(Ptr, *mut u32) -> u32;
    field_get_name: fn(Ptr) -> *const c_char;
    field_get_type: fn(Ptr) -> Ptr;
    field_get_flags: fn(Ptr) -> i32;
    field_static_set_value: fn(Ptr, *mut c_void);
    type_get_name: fn(Ptr) -> *mut c_char;
    free: fn(*mut c_void);
    image_get_name: fn(Ptr) -> *const c_char;
    image_get_class_count: fn(Ptr) -> usize;
    image_get_class: fn(Ptr, usize) -> Ptr;
    gchandle_new: fn(Ptr, bool) -> u32;
    gchandle_get_target: fn(u32) -> Ptr;
    gchandle_free: fn(u32);
    resolve_icall: fn(*const c_char) -> Ptr;
    array_length: fn(Ptr) -> u32;
    array_new: fn(Ptr, usize) -> Ptr;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field(pub Ptr);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image(pub Ptr);

unsafe impl Send for Field {}
unsafe impl Send for Image {}

pub const FIELD_STATIC: i32 = 0x10;
pub const FIELD_LITERAL: i32 = 0x40;
pub const METHOD_STATIC: u32 = 0x10;
pub const METHOD_VIRTUAL: u32 = 0x40;
pub const ARRAY_DATA_OFFSET: usize = 4 * size_of::<usize>();

fn text(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned()
    }
}

#[derive(Debug)]
pub struct GcHandle {
    api: Il2Cpp,
    handle: u32,
}

impl GcHandle {
    #[must_use]
    pub fn target(&self) -> Object {
        Object(unsafe { (self.api.gchandle_get_target)(self.handle) })
    }
}

impl Drop for GcHandle {
    fn drop(&mut self) {
        unsafe { (self.api.gchandle_free)(self.handle) };
    }
}

impl Il2Cpp {
    #[must_use]
    pub fn images(&self) -> Vec<Image> {
        let mut count = 0usize;
        let assemblies =
            unsafe { (self.domain_get_assemblies)((self.domain_get)(), &raw mut count) };
        if assemblies.is_null() {
            return Vec::new();
        }
        (0..count)
            .map(|i| Image(unsafe { (self.assembly_get_image)(*assemblies.add(i)) }))
            .filter(|i| !i.0.is_null())
            .collect()
    }

    #[must_use]
    pub fn image_name(&self, image: Image) -> String {
        text(unsafe { (self.image_get_name)(image.0) })
    }

    #[must_use]
    pub fn classes(&self, image: Image) -> Vec<Class> {
        let count = unsafe { (self.image_get_class_count)(image.0) };
        (0..count)
            .map(|i| Class(unsafe { (self.image_get_class)(image.0, i) }))
            .filter(|c| !c.0.is_null())
            .collect()
    }

    #[must_use]
    pub fn full_class_name(&self, class: Class) -> String {
        self.type_name(unsafe { (self.class_get_type)(class.0) })
    }

    #[must_use]
    pub fn class_name(&self, class: Class) -> String {
        text(unsafe { (self.class_get_name)(class.0) })
    }

    #[must_use]
    pub fn class_namespace(&self, class: Class) -> String {
        text(unsafe { (self.class_get_namespace)(class.0) })
    }

    #[must_use]
    pub fn parent(&self, class: Class) -> Option<Class> {
        let p = unsafe { (self.class_get_parent)(class.0) };
        (!p.is_null()).then_some(Class(p))
    }

    #[must_use]
    pub fn class_flags(&self, class: Class) -> i32 {
        unsafe { (self.class_get_flags)(class.0) }
    }

    #[must_use]
    pub fn is_value_type(&self, class: Class) -> bool {
        unsafe { (self.class_is_valuetype)(class.0) }
    }

    #[must_use]
    pub fn class_of(&self, object: Object) -> Option<Class> {
        if object.is_null() {
            return None;
        }
        let c = unsafe { (self.object_get_class)(object.0) };
        (!c.is_null()).then_some(Class(c))
    }

    #[must_use]
    pub fn methods(&self, class: Class) -> Vec<Method> {
        let mut iter: Ptr = std::ptr::null_mut();
        std::iter::from_fn(|| {
            let m = unsafe { (self.class_get_methods)(class.0, &raw mut iter) };
            (!m.is_null()).then_some(Method(m))
        })
        .collect()
    }

    #[must_use]
    pub fn fields(&self, class: Class) -> Vec<Field> {
        let mut iter: Ptr = std::ptr::null_mut();
        std::iter::from_fn(|| {
            let f = unsafe { (self.class_get_fields)(class.0, &raw mut iter) };
            (!f.is_null()).then_some(Field(f))
        })
        .collect()
    }

    #[must_use]
    pub fn method_name(&self, method: Method) -> String {
        text(unsafe { (self.method_get_name)(method.0) })
    }

    #[must_use]
    pub fn method_flags(&self, method: Method) -> u32 {
        let mut impl_flags = 0u32;
        unsafe { (self.method_get_flags)(method.0, &raw mut impl_flags) }
    }

    #[must_use]
    pub fn method_params(&self, method: Method) -> Vec<(String, String)> {
        let count = unsafe { (self.method_get_param_count)(method.0) };
        (0..count)
            .map(|i| {
                let ty = unsafe { (self.method_get_param)(method.0, i) };
                (
                    self.type_name(ty),
                    text(unsafe { (self.method_get_param_name)(method.0, i) }),
                )
            })
            .collect()
    }

    #[must_use]
    pub fn method_return_type(&self, method: Method) -> String {
        self.type_name(unsafe { (self.method_get_return_type)(method.0) })
    }

    #[must_use]
    fn type_name(&self, ty: Ptr) -> String {
        if ty.is_null() {
            return String::new();
        }
        let raw = unsafe { (self.type_get_name)(ty) };
        let name = text(raw);
        if !raw.is_null() {
            unsafe { (self.free)(raw.cast()) };
        }
        name
    }

    #[must_use]
    pub fn field(&self, class: Class, name: &str) -> Option<Field> {
        let name = CString::new(name).ok()?;
        let f = unsafe { (self.class_get_field_from_name)(class.0, name.as_ptr()) };
        (!f.is_null()).then_some(Field(f))
    }

    #[must_use]
    pub fn field_name(&self, field: Field) -> String {
        text(unsafe { (self.field_get_name)(field.0) })
    }

    #[must_use]
    pub fn field_type(&self, field: Field) -> String {
        self.type_name(unsafe { (self.field_get_type)(field.0) })
    }

    #[must_use]
    pub fn field_flags(&self, field: Field) -> i32 {
        unsafe { (self.field_get_flags)(field.0) }
    }

    #[must_use]
    pub fn offset(&self, field: Field) -> usize {
        unsafe { (self.field_get_offset)(field.0) }
    }

    #[must_use]
    pub unsafe fn get<T: Copy>(&self, object: Object, field: Field) -> T {
        unsafe {
            object
                .0
                .cast::<u8>()
                .add(self.offset(field))
                .cast::<T>()
                .read_unaligned()
        }
    }

    pub unsafe fn set<T: Copy>(&self, object: Object, field: Field, value: T) {
        unsafe {
            object
                .0
                .cast::<u8>()
                .add(self.offset(field))
                .cast::<T>()
                .write_unaligned(value);
        }
    }

    #[must_use]
    pub unsafe fn get_static<T: Copy + Default>(&self, field: Field) -> T {
        let mut value = T::default();
        unsafe { (self.field_static_get_value)(field.0, (&raw mut value).cast()) };
        value
    }

    pub unsafe fn set_static<T: Copy>(&self, field: Field, mut value: T) {
        unsafe { (self.field_static_set_value)(field.0, (&raw mut value).cast()) };
    }

    #[must_use]
    pub fn new_object(&self, class: Class) -> Object {
        Object(unsafe { (self.object_new)(class.0) })
    }

    #[must_use]
    pub fn pin(&self, object: Object) -> GcHandle {
        GcHandle {
            api: *self,
            handle: unsafe { (self.gchandle_new)(object.0, false) },
        }
    }

    #[must_use]
    pub fn icall(&self, signature: &str) -> Option<Ptr> {
        let s = CString::new(signature).ok()?;
        let p = unsafe { (self.resolve_icall)(s.as_ptr()) };
        (!p.is_null()).then_some(p)
    }

    #[must_use]
    pub fn new_array(&self, element: Class, len: usize) -> Object {
        Object(unsafe { (self.array_new)(element.0, len) })
    }

    #[must_use]
    pub unsafe fn array<T: Copy>(&self, array: Object) -> &[T] {
        if array.is_null() {
            return &[];
        }
        let len = unsafe { (self.array_length)(array.0) } as usize;
        unsafe {
            std::slice::from_raw_parts(array.0.cast::<u8>().add(ARRAY_DATA_OFFSET).cast::<T>(), len)
        }
    }
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
