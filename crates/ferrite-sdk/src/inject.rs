use crate::Context;
use crate::abi;
use crate::il2cpp::{Class, Il2Cpp, Object, Ptr};
use std::ffi::{CStr, CString, c_char, c_void};
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

const IMAGE: usize = 0x00;
const NAME: usize = 0x10;
const NAMESPACE: usize = 0x18;
const BYVAL_ARG: usize = 0x20;
const THIS_ARG: usize = 0x30;
const ELEMENT_CLASS: usize = 0x40;
const CAST_CLASS: usize = 0x48;
const PARENT: usize = 0x58;
const KLASS: usize = 0x78;
const FIELDS: usize = 0x80;
const METHODS: usize = 0x98;
const IMPLEMENTED_INTERFACES: usize = 0xA8;
const INTERFACE_OFFSETS: usize = 0xB0;
const TYPE_HIERARCHY: usize = 0xC8;
const CCTOR_FINISHED: usize = 0xE0;
const INSTANCE_SIZE: usize = 0xF8;
const VTABLE: usize = 0x138;
const VTABLE_ENTRY: usize = 0x10;

const TYPE_SIZE: usize = 0x10;
const TYPE_ATTRS: usize = 0x08;

const M_POINTER: usize = 0x00;
const M_VIRTUAL_POINTER: usize = 0x08;
const M_INVOKER: usize = 0x10;
const M_NAME: usize = 0x18;
const M_CLASS: usize = 0x20;
const M_RETURN: usize = 0x28;
const M_PARAMETERS: usize = 0x30;
const M_TOKEN: usize = 0x48;
const M_FLAGS: usize = 0x4C;
const M_SLOT: usize = 0x50;
const M_PARAM_COUNT: usize = 0x52;
const METHOD_SIZE: usize = 0x58;

const F_NAME: usize = 0x00;
const F_TYPE: usize = 0x08;
const F_PARENT: usize = 0x10;
const F_OFFSET: usize = 0x18;
const FIELD_SIZE: usize = 0x20;

const BF0_INITIALIZED: u8 = 0x03;
const BF0_GENERIC: u8 = 0x10;
const BF0_HAS_REFERENCES: u8 = 0x20;
const BF1_SIZE_INITED: u8 = 0x01;
const BF1_VTABLE_INITIALIZED: u8 = 0x20;

const CLASS_INTERFACE: i32 = 0x20;
const CLASS_ABSTRACT: i32 = 0x80;
const CLASS_SEALED: i32 = 0x100;
const METHOD_PUBLIC_HIDE_BY_SIG: u16 = 0x0086;
const METHOD_CTOR: u16 = 0x1886;
const FIELD_PUBLIC_NOT_SERIALIZED: u16 = 0x0086;
const METADATA_MAGIC: u32 = 0xFAB1_1BAF;
const MIN_METADATA: u32 = 29;
const MAX_TESTED_METADATA: u32 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    name: &'static str,
    stack_slot: bool,
}

const LAYOUTS: [Layout; 2] = [
    Layout {
        name: "2022.2+",
        stack_slot: true,
    },
    Layout {
        name: "2021.2-2022.1",
        stack_slot: false,
    },
];

impl Layout {
    const fn at(self, offset: usize) -> usize {
        if self.stack_slot { offset + 4 } else { offset }
    }

    const fn actual_size(self) -> usize {
        self.at(0xFC)
    }
    const fn native_size(self) -> usize {
        self.at(0x104)
    }
    const fn flags(self) -> usize {
        self.at(0x114)
    }
    const fn token(self) -> usize {
        self.at(0x118)
    }
    const fn method_count(self) -> usize {
        self.at(0x11C)
    }
    const fn field_count(self) -> usize {
        self.at(0x120)
    }
    const fn vtable_count(self) -> usize {
        self.at(0x126)
    }
    const fn interfaces_count(self) -> usize {
        self.at(0x128)
    }
    const fn interface_offsets_count(self) -> usize {
        self.at(0x12A)
    }
    const fn depth(self) -> usize {
        self.at(0x12C)
    }
    const fn minimum_alignment(self) -> usize {
        self.at(0x12F)
    }
    const fn natural_alignment(self) -> Option<usize> {
        if self.stack_slot { None } else { Some(0x130) }
    }
    const fn stack_slot_size(self) -> Option<usize> {
        if self.stack_slot { Some(0xFC) } else { None }
    }
    const fn packing_size(self) -> usize {
        if self.stack_slot { 0x134 } else { 0x131 }
    }
    const fn bitfield0(self) -> usize {
        if self.stack_slot { 0x135 } else { 0x132 }
    }
    const fn bitfield1(self) -> usize {
        self.bitfield0() + 1
    }
}

#[derive(Debug)]
pub enum InjectError {
    NoIl2Cpp,
    MissingExport(&'static str),
    UnsupportedMetadata(u32),
    UnknownLayout(String),
    InvalidParent(String),
    TypeNotFound(String),
    InvalidName(String),
    Changed(String),
    Registration,
}

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoIl2Cpp => {
                f.write_str("IL2CPP API unavailable, class injection needs an IL2CPP game")
            }
            Self::MissingExport(name) => write!(f, "{name} is not exported by GameAssembly.dll"),
            Self::UnsupportedMetadata(v) => write!(
                f,
                "IL2CPP metadata v{v} is not supported, class injection needs v{MIN_METADATA} or newer (Unity 2021.2+)"
            ),
            Self::UnknownLayout(e) => write!(f, "unknown Il2CppClass layout ({e})"),
            Self::InvalidParent(e) => write!(f, "cannot inherit: {e}"),
            Self::TypeNotFound(t) => write!(f, "type {t} not found"),
            Self::InvalidName(n) => write!(f, "invalid name {n:?}"),
            Self::Changed(e) => write!(f, "{e}, restart the game to apply this change"),
            Self::Registration => f.write_str("the loader refused the class, see the log above"),
        }
    }
}

impl std::error::Error for InjectError {}

unsafe fn rd<T: Copy>(base: Ptr, offset: usize) -> T {
    unsafe { base.cast::<u8>().add(offset).cast::<T>().read_unaligned() }
}

unsafe fn wr<T: Copy>(base: Ptr, offset: usize, value: T) {
    unsafe {
        base.cast::<u8>()
            .add(offset)
            .cast::<T>()
            .write_unaligned(value)
    };
}

fn alloc(size: usize) -> Ptr {
    Box::leak(vec![0u64; size.div_ceil(8)].into_boxed_slice())
        .as_mut_ptr()
        .cast()
}

fn leak_str(s: &str) -> Result<*const c_char, InjectError> {
    CString::new(s)
        .map(|c| c.into_raw().cast_const())
        .map_err(|_| InjectError::InvalidName(s.to_owned()))
}

unsafe fn text(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone, Copy)]
struct Extra {
    instance_size: unsafe extern "C" fn(Ptr) -> i32,
    type_token: unsafe extern "C" fn(Ptr) -> u32,
    method_class: unsafe extern "C" fn(Ptr) -> Ptr,
    method_token: unsafe extern "C" fn(Ptr) -> u32,
    field_parent: unsafe extern "C" fn(Ptr) -> Ptr,
    class_init: unsafe extern "C" fn(Ptr),
    has_references: unsafe extern "C" fn(Ptr) -> bool,
    value_size: unsafe extern "C" fn(Ptr, *mut u32) -> i32,
    class_image: unsafe extern "C" fn(Ptr) -> Ptr,
}

impl Extra {
    fn load(ctx: &Context) -> Result<Self, InjectError> {
        fn get<T: Copy>(ctx: &Context, name: &'static str) -> Result<T, InjectError> {
            let p = ctx.symbol(name).ok_or(InjectError::MissingExport(name))?;
            Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&p) })
        }
        Ok(Self {
            instance_size: get(ctx, "il2cpp_class_instance_size")?,
            type_token: get(ctx, "il2cpp_class_get_type_token")?,
            method_class: get(ctx, "il2cpp_method_get_class")?,
            method_token: get(ctx, "il2cpp_method_get_token")?,
            field_parent: get(ctx, "il2cpp_field_get_parent")?,
            class_init: get(ctx, "il2cpp_runtime_class_init")?,
            has_references: get(ctx, "il2cpp_class_has_references")?,
            value_size: get(ctx, "il2cpp_class_value_size")?,
            class_image: get(ctx, "il2cpp_class_get_image")?,
        })
    }
}

fn expect(ok: bool, what: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(what()) }
}

unsafe fn initialized(l: Layout, class: Class) -> bool {
    let bits = unsafe { rd::<u8>(class.0, l.bitfield0()) };
    bits & BF0_INITIALIZED == BF0_INITIALIZED
}

unsafe fn ensure_init(api: &Il2Cpp, ex: &Extra, l: Layout, class: Class) {
    unsafe {
        (ex.class_init)(class.0);
        if initialized(l, class) {
            return;
        }
        let _ = (ex.has_references)(class.0);
        if initialized(l, class)
            || api.class_flags(class) & (CLASS_ABSTRACT | CLASS_INTERFACE | CLASS_SEALED) != 0
        {
            return;
        }
        let _ = api.new_object(class);
    }
}

unsafe fn probe_scalars(api: &Il2Cpp, ex: &Extra, l: Layout, class: Class) -> Result<(), String> {
    let k = class.0;
    unsafe { ensure_init(api, ex, l, class) };
    let name = api.class_name(class);
    unsafe {
        expect(
            rd::<Ptr>(k, NAME) == (api.class_get_name)(k).cast_mut().cast(),
            || format!("{name}: name"),
        )?;
        expect(
            rd::<Ptr>(k, NAMESPACE) == (api.class_get_namespace)(k).cast_mut().cast(),
            || format!("{name}: namespace"),
        )?;
        expect(
            k.cast::<u8>().add(BYVAL_ARG).cast() == (api.class_get_type)(k),
            || format!("{name}: byval_arg"),
        )?;
        expect(rd::<Ptr>(k, PARENT) == (api.class_get_parent)(k), || {
            format!("{name}: parent")
        })?;
        expect(
            rd::<u32>(k, INSTANCE_SIZE).cast_signed() == (ex.instance_size)(k),
            || format!("{name}: instance_size"),
        )?;
        expect(
            rd::<u32>(k, l.flags()).cast_signed() == api.class_flags(class),
            || format!("{name}: flags"),
        )?;
        expect(rd::<u32>(k, l.token()) == (ex.type_token)(k), || {
            format!("{name}: token")
        })?;
        expect(
            usize::from(rd::<u16>(k, l.method_count())) == api.methods(class).len(),
            || format!("{name}: method_count"),
        )?;
        expect(
            usize::from(rd::<u16>(k, l.field_count())) == api.fields(class).len(),
            || format!("{name}: field_count"),
        )?;
        expect(
            rd::<u8>(k, l.bitfield0()) & BF0_INITIALIZED == BF0_INITIALIZED
                && rd::<u8>(k, l.bitfield1()) & BF1_SIZE_INITED != 0,
            || {
                format!(
                    "{name}: init flags {:#x} {:#x}",
                    rd::<u8>(k, l.bitfield0()),
                    rd::<u8>(k, l.bitfield1())
                )
            },
        )?;
        let depth = rd::<u8>(k, l.depth());
        expect((1..64).contains(&depth), || {
            format!("{name}: hierarchy depth")
        })?;
    }
    Ok(())
}

unsafe fn probe_pointers(api: &Il2Cpp, ex: &Extra, l: Layout, class: Class) -> Result<(), String> {
    let k = class.0;
    let name = api.class_name(class);
    unsafe {
        let depth = usize::from(rd::<u8>(k, l.depth()));
        let hierarchy = rd::<*const Ptr>(k, TYPE_HIERARCHY);
        expect(
            !hierarchy.is_null() && *hierarchy.add(depth - 1) == k,
            || format!("{name}: type hierarchy"),
        )?;
        let ancestors = std::slice::from_raw_parts(hierarchy, depth);
        for i in 0..usize::from(rd::<u16>(k, l.vtable_count())).min(512) {
            let entry = VTABLE + i * VTABLE_ENTRY;
            let method = rd::<Ptr>(k, entry + 8);
            if method.is_null() {
                continue;
            }
            expect(
                ancestors.contains(&(ex.method_class)(method))
                    && rd::<Ptr>(k, entry) == rd::<Ptr>(method, M_POINTER),
                || format!("{name}: vtable[{i}]"),
            )?;
        }
        for method in api.methods(class) {
            let m = method.0;
            let count = (api.method_get_param_count)(m);
            let mut flags = 0u32;
            let real_flags = (api.method_get_flags)(m, &raw mut flags);
            let params = rd::<*const Ptr>(m, M_PARAMETERS);
            let params_ok = (0..count).all(|i| {
                !params.is_null() && *params.add(i as usize) == (api.method_get_param)(m, i)
            });
            expect(
                text(rd(m, M_NAME)) == api.method_name(method)
                    && rd::<Ptr>(m, M_CLASS) == (ex.method_class)(m)
                    && rd::<Ptr>(m, M_RETURN) == (api.method_get_return_type)(m)
                    && u32::from(rd::<u8>(m, M_PARAM_COUNT)) == count
                    && rd::<u32>(m, M_TOKEN) == (ex.method_token)(m)
                    && u32::from(rd::<u16>(m, M_FLAGS)) == real_flags
                    && params_ok,
                || format!("{name}.{}: MethodInfo", api.method_name(method)),
            )?;
        }
        for field in api.fields(class) {
            let f = field.0;
            expect(
                text(rd(f, F_NAME)) == api.field_name(field)
                    && rd::<Ptr>(f, F_TYPE) == (api.field_get_type)(f)
                    && rd::<Ptr>(f, F_PARENT) == (ex.field_parent)(f)
                    && usize::try_from(rd::<i32>(f, F_OFFSET)).ok() == Some(api.offset(field)),
                || format!("{name}.{}: FieldInfo", api.field_name(field)),
            )?;
        }
    }
    Ok(())
}

fn detect(api: &Il2Cpp, ex: &Extra, probes: &[Class]) -> Result<Layout, String> {
    for &class in probes {
        unsafe { (ex.class_init)(class.0) };
    }
    let mut errors = Vec::new();
    for layout in LAYOUTS {
        let result = probes
            .iter()
            .try_for_each(|&c| unsafe { probe_scalars(api, ex, layout, c) })
            .and_then(|()| {
                probes
                    .iter()
                    .try_for_each(|&c| unsafe { probe_pointers(api, ex, layout, c) })
            });
        match result {
            Ok(()) => return Ok(layout),
            Err(e) => errors.push(format!("{} mismatch at {e}", layout.name)),
        }
    }
    Err(errors.join(", "))
}

fn layout(ctx: &Context, api: &Il2Cpp, ex: &Extra) -> Result<Layout, InjectError> {
    static DETECTED: OnceLock<Result<Layout, String>> = OnceLock::new();
    DETECTED
        .get_or_init(|| {
            let probes: Vec<Class> = [
                ("System", "Object"),
                ("System", "String"),
                ("UnityEngine", "Object"),
                ("UnityEngine", "MonoBehaviour"),
            ]
            .iter()
            .filter_map(|(ns, n)| api.class(ns, n))
            .collect();
            let layout = detect(api, ex, &probes)?;
            ctx.debug(&format!(
                "Il2CppClass layout {} verified on {} classes",
                layout.name,
                probes.len()
            ));
            Ok(layout)
        })
        .clone()
        .map_err(InjectError::UnknownLayout)
}

#[must_use]
pub fn metadata_version(game_dir: &Path) -> Option<u32> {
    std::fs::read_dir(game_dir).ok()?.flatten().find_map(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.ends_with("_Data") {
            return None;
        }
        let file = e
            .path()
            .join("il2cpp_data")
            .join("Metadata")
            .join("global-metadata.dat");
        let mut header = [0u8; 8];
        std::io::Read::read_exact(&mut std::fs::File::open(file).ok()?, &mut header).ok()?;
        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        (magic == METADATA_MAGIC)
            .then(|| u32::from_le_bytes([header[4], header[5], header[6], header[7]]))
    })
}

pub trait Value: Copy + 'static {
    const TYPE: (&'static str, &'static str);
    unsafe fn load(arg: Ptr) -> Self;
    unsafe fn store(self, ret: Ptr);
}

impl Value for () {
    const TYPE: (&'static str, &'static str) = ("System", "Void");
    unsafe fn load(_: Ptr) -> Self {}
    unsafe fn store(self, _: Ptr) {}
}

impl Value for Object {
    const TYPE: (&'static str, &'static str) = ("System", "Object");
    unsafe fn load(arg: Ptr) -> Self {
        Object(arg)
    }
    unsafe fn store(self, ret: Ptr) {
        if !ret.is_null() {
            unsafe { ret.cast::<Ptr>().write_unaligned(self.0) };
        }
    }
}

macro_rules! value_types {
    ($($ty:ty => $name:literal),*) => {
        $(impl Value for $ty {
            const TYPE: (&'static str, &'static str) = ("System", $name);
            unsafe fn load(arg: Ptr) -> Self {
                unsafe { arg.cast::<$ty>().read_unaligned() }
            }
            unsafe fn store(self, ret: Ptr) {
                if !ret.is_null() {
                    unsafe { ret.cast::<$ty>().write_unaligned(self) };
                }
            }
        })*
    };
}

value_types!(
    bool => "Boolean", i8 => "SByte", u8 => "Byte", i16 => "Int16", u16 => "UInt16",
    i32 => "Int32", u32 => "UInt32", i64 => "Int64", u64 => "UInt64",
    f32 => "Single", f64 => "Double", Ptr => "IntPtr"
);

#[derive(Debug, Clone)]
struct MethodDef {
    name: String,
    pointer: Ptr,
    invoker: Ptr,
    ret: (&'static str, &'static str),
    params: Vec<(&'static str, &'static str)>,
}

#[derive(Debug)]
pub struct ClassBuilder<'a> {
    ctx: &'a Context,
    namespace: String,
    name: String,
    parent: (String, String),
    fields: Vec<(String, String, String)>,
    methods: Vec<MethodDef>,
}

macro_rules! method_variant {
    ($method:ident, $invoker:ident, $($arg:ident: $index:tt),*) => {
        unsafe extern "C" fn $invoker<R: Value, $($arg: Value),*>(
            f: Ptr,
            method: Ptr,
            obj: Ptr,
            args: *mut Ptr,
            ret: Ptr,
        ) {
            let _ = args;
            let f = unsafe {
                std::mem::transmute::<Ptr, unsafe extern "C" fn(Object, $($arg,)* Ptr) -> R>(f)
            };
            let result = unsafe { f(Object(obj), $(<$arg as Value>::load(*args.add($index)),)* method) };
            unsafe { result.store(ret) };
        }

        impl ClassBuilder<'_> {
            #[must_use]
            pub fn $method<R: Value, $($arg: Value),*>(
                mut self,
                name: &str,
                f: unsafe extern "C" fn(Object, $($arg,)* Ptr) -> R,
            ) -> Self {
                self.methods.push(MethodDef {
                    name: name.to_owned(),
                    pointer: f as Ptr,
                    invoker: $invoker::<R, $($arg),*> as *const () as Ptr,
                    ret: R::TYPE,
                    params: vec![$(<$arg as Value>::TYPE),*],
                });
                self
            }
        }
    };
}

method_variant!(method, invoke0,);
method_variant!(method1, invoke1, A: 0);
method_variant!(method2, invoke2, A: 0, B: 1);
method_variant!(method3, invoke3, A: 0, B: 1, C: 2);

static CTOR_API: OnceLock<Il2Cpp> = OnceLock::new();

unsafe extern "C" fn base_ctor(this: Object, method: Ptr) {
    let Some(api) = CTOR_API.get() else { return };
    let parent = unsafe { rd::<Ptr>(rd::<Ptr>(method, M_CLASS), PARENT) };
    if let Some(ctor) = api.method(Class(parent), ".ctor", 0) {
        let pointer = ctor.pointer();
        if !pointer.is_null() {
            let f =
                unsafe { std::mem::transmute::<Ptr, unsafe extern "C" fn(Object, Ptr)>(pointer) };
            unsafe { f(this, ctor.0) };
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Injected {
    pub class: Class,
    pub token: i64,
    pub instance_size: usize,
}

impl Injected {
    #[must_use]
    pub fn type_object(&self, api: &Il2Cpp) -> Object {
        api.type_object(self.class)
    }
}

struct Built {
    class: Ptr,
    methods: Vec<(Ptr, MethodDef)>,
}

impl<'a> ClassBuilder<'a> {
    #[must_use]
    pub fn new(ctx: &'a Context, namespace: &str, name: &str) -> Self {
        Self {
            ctx,
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            parent: ("System".to_owned(), "Object".to_owned()),
            fields: Vec::new(),
            methods: Vec::new(),
        }
    }

    #[must_use]
    pub fn parent(mut self, namespace: &str, name: &str) -> Self {
        self.parent = (namespace.to_owned(), name.to_owned());
        self
    }

    #[must_use]
    pub fn field(mut self, name: &str, namespace: &str, type_name: &str) -> Self {
        self.fields
            .push((name.to_owned(), namespace.to_owned(), type_name.to_owned()));
        self
    }

    #[must_use]
    pub fn field_of<T: Value>(self, name: &str) -> Self {
        let (namespace, type_name) = T::TYPE;
        self.field(name, namespace, type_name)
    }

    pub fn build(self) -> Result<Injected, InjectError> {
        let ctx = self.ctx;
        let api = ctx.il2cpp().ok_or(InjectError::NoIl2Cpp)?;
        let metadata = metadata_version(&ctx.game_dir());
        if let Some(v) = metadata.filter(|&v| v < MIN_METADATA) {
            return Err(InjectError::UnsupportedMetadata(v));
        }
        if let Some(v) = metadata.filter(|&v| v > MAX_TESTED_METADATA) {
            ctx.warning(&format!(
                "IL2CPP metadata v{v} is newer than tested (v{MIN_METADATA}..v{MAX_TESTED_METADATA}), relying on layout checks"
            ));
        }
        let ex = Extra::load(ctx)?;
        let _ = CTOR_API.set(api);
        let find = |ns: &str, n: &str| {
            api.class(ns, n)
                .ok_or_else(|| InjectError::TypeNotFound(full_name(ns, n)))
        };
        let parent = find(&self.parent.0, &self.parent.1)?;
        unsafe { (ex.class_init)(parent.0) };
        let layout = layout(ctx, &api, &ex)?;
        let mut probes = vec![parent];
        probes.extend(api.parent(parent));
        detect_one(&api, &ex, layout, &probes).map_err(InjectError::UnknownLayout)?;
        check_parent(&api, layout, parent)?;
        if let Some(existing) = api.class(&self.namespace, &self.name) {
            return self.rebind(&api, layout, parent, existing);
        }
        let built = unsafe { self.assemble(&api, &ex, layout, parent, &find) }?;
        let namespace = leak_str(&self.namespace)?;
        let name = leak_str(&self.name)?;
        let token = unsafe { (ctx.api.register_class)(ctx.handle, built.class, namespace, name) };
        if token >= 0 {
            return Err(InjectError::Registration);
        }
        unsafe {
            wr(built.class, BYVAL_ARG, token);
            wr(built.class, THIS_ARG, token);
        }
        for (info, def) in &built.methods {
            unsafe { bind(ctx, *info, def) };
        }
        let injected = Injected {
            class: Class(built.class),
            token,
            instance_size: unsafe { rd::<u32>(built.class, INSTANCE_SIZE) } as usize,
        };
        ctx.info(&format!(
            "injected class {} : {} (layout {}, metadata {}, {} method(s), {} field(s), instance size {:#x})",
            full_name(&self.namespace, &self.name),
            full_name(&self.parent.0, &self.parent.1),
            layout.name,
            metadata.map_or_else(|| "unknown".to_owned(), |v| format!("v{v}")),
            built.methods.len(),
            self.fields.len(),
            injected.instance_size
        ));
        Ok(injected)
    }

    fn defs(&self) -> Vec<MethodDef> {
        let mut defs = vec![MethodDef {
            name: ".ctor".to_owned(),
            pointer: base_ctor as *const () as Ptr,
            invoker: invoke0::<()> as *const () as Ptr,
            ret: <() as Value>::TYPE,
            params: Vec::new(),
        }];
        defs.extend(self.methods.iter().cloned());
        defs
    }

    fn rebind(
        &self,
        api: &Il2Cpp,
        l: Layout,
        parent: Class,
        existing: Class,
    ) -> Result<Injected, InjectError> {
        let full = full_name(&self.namespace, &self.name);
        let k = existing.0;
        let token = unsafe { rd::<i64>(k, BYVAL_ARG) };
        if token >= 0 {
            return Err(InjectError::InvalidName(format!(
                "{full} already exists in the game"
            )));
        }
        if api.parent(existing) != Some(parent) {
            return Err(InjectError::Changed(format!(
                "{full} was injected with another parent"
            )));
        }
        let fields: Vec<String> = api
            .fields(existing)
            .into_iter()
            .map(|f| api.field_name(f))
            .collect();
        let wanted: Vec<String> = self.fields.iter().map(|f| f.0.clone()).collect();
        if fields != wanted {
            return Err(InjectError::Changed(format!(
                "fields of {full} changed from {fields:?} to {wanted:?}"
            )));
        }
        let count = usize::from(unsafe { rd::<u16>(k, l.method_count()) });
        let table = unsafe { rd::<*const Ptr>(k, METHODS) };
        let methods: Vec<Ptr> = (0..count).map(|i| unsafe { *table.add(i) }).collect();
        let mut bound = Vec::new();
        for def in self.defs() {
            let info = methods.iter().copied().find(|&m| unsafe {
                text(rd(m, M_NAME)) == def.name
                    && usize::from(rd::<u8>(m, M_PARAM_COUNT)) == def.params.len()
            });
            let Some(info) = info else {
                return Err(InjectError::Changed(format!(
                    "{full} has no method {} with {} parameter(s) yet",
                    def.name,
                    def.params.len()
                )));
            };
            bound.push((info, def));
        }
        for (info, def) in &bound {
            unsafe { bind(self.ctx, *info, def) };
        }
        let injected = Injected {
            class: existing,
            token,
            instance_size: unsafe { rd::<u32>(k, INSTANCE_SIZE) } as usize,
        };
        self.ctx.info(&format!(
            "{full} is already injected, its {} method(s) now point to this build",
            bound.len()
        ));
        Ok(injected)
    }

    unsafe fn assemble(
        &self,
        api: &Il2Cpp,
        ex: &Extra,
        l: Layout,
        parent: Class,
        find: &dyn Fn(&str, &str) -> Result<Class, InjectError>,
    ) -> Result<Built, InjectError> {
        let p = parent.0;
        let vtable_count = unsafe { rd::<u16>(p, l.vtable_count()) };
        let k = alloc(VTABLE + usize::from(vtable_count) * VTABLE_ENTRY);
        let image = api
            .images()
            .into_iter()
            .find(|i| api.image_name(*i) == "Assembly-CSharp.dll")
            .map_or_else(|| unsafe { (ex.class_image)(p) }, |i| i.0);
        let mut has_references = unsafe { rd::<u8>(p, l.bitfield0()) } & BF0_HAS_REFERENCES != 0;
        let mut offset = unsafe { rd::<u32>(p, INSTANCE_SIZE) } as usize;
        let fields = alloc(self.fields.len().max(1) * FIELD_SIZE);
        for (i, (field_name, ns, type_name)) in self.fields.iter().enumerate() {
            let class = find(ns, type_name)?;
            let (size, align) = if api.is_value_type(class) {
                let mut align = 0u32;
                let size = unsafe { (ex.value_size)(class.0, &raw mut align) };
                let size = usize::try_from(size).unwrap_or(8).max(1);
                (size, size.next_power_of_two().min(8))
            } else {
                has_references = true;
                (8, 8)
            };
            offset = offset.next_multiple_of(align);
            let ty = alloc(TYPE_SIZE);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    (api.class_get_type)(class.0).cast::<u8>(),
                    ty.cast::<u8>(),
                    TYPE_SIZE,
                );
                wr(ty, TYPE_ATTRS, FIELD_PUBLIC_NOT_SERIALIZED);
                let f = fields.cast::<u8>().add(i * FIELD_SIZE).cast::<c_void>();
                wr(f, F_NAME, leak_str(field_name)?);
                wr(f, F_TYPE, ty);
                wr(f, F_PARENT, k);
                wr(f, F_OFFSET, i32::try_from(offset).unwrap_or(i32::MAX));
            }
            offset += size;
        }
        let instance_size = offset.next_multiple_of(8);
        let defs = self.defs();
        let table = alloc(defs.len() * 8).cast::<Ptr>();
        let mut methods = Vec::with_capacity(defs.len());
        for (i, def) in defs.into_iter().enumerate() {
            let m = alloc(METHOD_SIZE);
            let ret = find(def.ret.0, def.ret.1)?;
            let params = alloc(def.params.len().max(1) * 8).cast::<Ptr>();
            for (j, (ns, n)) in def.params.iter().enumerate() {
                let class = find(ns, n)?;
                unsafe { params.add(j).write((api.class_get_type)(class.0)) };
            }
            unsafe {
                wr(m, M_NAME, leak_str(&def.name)?);
                wr(m, M_CLASS, k);
                wr(m, M_RETURN, (api.class_get_type)(ret.0));
                wr(m, M_PARAMETERS, params);
                wr(
                    m,
                    M_FLAGS,
                    if i == 0 {
                        METHOD_CTOR
                    } else {
                        METHOD_PUBLIC_HIDE_BY_SIG
                    },
                );
                wr(m, M_SLOT, u16::MAX);
                wr(
                    m,
                    M_PARAM_COUNT,
                    u8::try_from(def.params.len()).unwrap_or(u8::MAX),
                );
                table.add(i).write(m);
            }
            methods.push((m, def));
        }
        let depth = unsafe { rd::<u8>(p, l.depth()) };
        let hierarchy = alloc((usize::from(depth) + 1) * 8).cast::<Ptr>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                rd::<*const Ptr>(p, TYPE_HIERARCHY),
                hierarchy,
                usize::from(depth),
            );
            hierarchy.add(usize::from(depth)).write(k);
            std::ptr::copy_nonoverlapping(
                p.cast::<u8>().add(VTABLE),
                k.cast::<u8>().add(VTABLE),
                usize::from(vtable_count) * VTABLE_ENTRY,
            );
            std::ptr::copy_nonoverlapping(
                p.cast::<u8>().add(BYVAL_ARG),
                k.cast::<u8>().add(BYVAL_ARG),
                2 * TYPE_SIZE,
            );
            wr(k, IMAGE, image);
            wr(k, NAME, leak_str(&self.name)?);
            wr(k, NAMESPACE, leak_str(&self.namespace)?);
            wr(k, ELEMENT_CLASS, k);
            wr(k, CAST_CLASS, k);
            wr(k, PARENT, p);
            wr(k, KLASS, k);
            if !self.fields.is_empty() {
                wr(k, FIELDS, fields);
            }
            wr(k, METHODS, table);
            wr(
                k,
                IMPLEMENTED_INTERFACES,
                rd::<Ptr>(p, IMPLEMENTED_INTERFACES),
            );
            wr(k, INTERFACE_OFFSETS, rd::<Ptr>(p, INTERFACE_OFFSETS));
            wr(k, TYPE_HIERARCHY, hierarchy);
            wr(k, CCTOR_FINISHED, 1u32);
            let size = u32::try_from(instance_size).unwrap_or(u32::MAX);
            wr(k, INSTANCE_SIZE, size);
            wr(k, l.actual_size(), size);
            if let Some(at) = l.stack_slot_size() {
                wr(k, at, 8u32);
            }
            wr(k, l.native_size(), -1i32);
            wr(k, l.flags(), rd::<i32>(p, l.flags()) & !CLASS_ABSTRACT);
            wr(
                k,
                l.method_count(),
                u16::try_from(methods.len()).unwrap_or(u16::MAX),
            );
            wr(
                k,
                l.field_count(),
                u16::try_from(self.fields.len()).unwrap_or(u16::MAX),
            );
            wr(k, l.vtable_count(), vtable_count);
            wr(k, l.interfaces_count(), rd::<u16>(p, l.interfaces_count()));
            wr(
                k,
                l.interface_offsets_count(),
                rd::<u16>(p, l.interface_offsets_count()),
            );
            wr(k, l.depth(), depth + 1);
            wr(k, l.minimum_alignment(), rd::<u8>(p, l.minimum_alignment()));
            if let Some(at) = l.natural_alignment() {
                wr(k, at, rd::<u8>(p, at));
            }
            wr(k, l.packing_size(), rd::<u8>(p, l.packing_size()));
            let bf0 = BF0_INITIALIZED
                | if has_references {
                    BF0_HAS_REFERENCES
                } else {
                    0
                };
            wr(k, l.bitfield0(), bf0);
            wr(k, l.bitfield1(), BF1_SIZE_INITED | BF1_VTABLE_INITIALIZED);
        }
        Ok(Built { class: k, methods })
    }
}

unsafe fn bind(ctx: &Context, info: Ptr, def: &MethodDef) {
    unsafe {
        let stub_method = (ctx.api.stub)(abi::STUB_METHOD);
        let stub_invoker = (ctx.api.stub)(abi::STUB_INVOKER);
        for (slot, stub, value) in [
            (M_POINTER, stub_method, def.pointer),
            (M_VIRTUAL_POINTER, stub_method, def.pointer),
            (M_INVOKER, stub_invoker, def.invoker),
        ] {
            let slot = info.cast::<u8>().add(slot).cast::<Ptr>();
            slot.write_volatile(stub);
            (ctx.api.patch_pointer)(ctx.handle, slot, value);
        }
    }
}

fn detect_one(api: &Il2Cpp, ex: &Extra, layout: Layout, probes: &[Class]) -> Result<(), String> {
    probes.iter().try_for_each(|&c| unsafe {
        probe_scalars(api, ex, layout, c)?;
        probe_pointers(api, ex, layout, c)
    })
}

fn check_parent(api: &Il2Cpp, l: Layout, parent: Class) -> Result<(), InjectError> {
    let full = api.full_class_name(parent);
    let flags = api.class_flags(parent);
    let generic = unsafe { rd::<u8>(parent.0, l.bitfield0()) } & BF0_GENERIC != 0;
    let reason = if api.is_value_type(parent) {
        Some("is a value type")
    } else if flags & CLASS_INTERFACE != 0 {
        Some("is an interface")
    } else if flags & CLASS_SEALED != 0 {
        Some("is sealed")
    } else if generic {
        Some("is generic")
    } else {
        None
    };
    reason.map_or(Ok(()), |r| {
        Err(InjectError::InvalidParent(format!("{full} {r}")))
    })
}

fn full_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}.{name}")
    }
}
