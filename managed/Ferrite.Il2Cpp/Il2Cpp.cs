using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Ferrite {
    public static unsafe class Il2Cpp {
        public const int FieldStatic = 0x10;
        public const int FieldLiteral = 0x40;
        public const uint MethodStatic = 0x10;
        public const uint MethodVirtual = 0x40;
        public static readonly int ArrayDataOffset = 4 * IntPtr.Size;

        internal static delegate* unmanaged<IntPtr> domain_get;
        internal static delegate* unmanaged<IntPtr, nuint*, IntPtr*> domain_get_assemblies;
        internal static delegate* unmanaged<IntPtr, IntPtr> assembly_get_image;
        internal static delegate* unmanaged<IntPtr, byte*> image_get_name;
        internal static delegate* unmanaged<IntPtr, nuint> image_get_class_count;
        internal static delegate* unmanaged<IntPtr, nuint, IntPtr> image_get_class;
        internal static delegate* unmanaged<IntPtr, byte*, byte*, IntPtr> class_from_name;
        internal static delegate* unmanaged<IntPtr, byte*, int, IntPtr> class_get_method_from_name;
        internal static delegate* unmanaged<IntPtr, byte*, IntPtr> class_get_field_from_name;
        internal static delegate* unmanaged<IntPtr, byte*> class_get_name;
        internal static delegate* unmanaged<IntPtr, byte*> class_get_namespace;
        internal static delegate* unmanaged<IntPtr, IntPtr> class_get_parent;
        internal static delegate* unmanaged<IntPtr, IntPtr> class_get_type;
        internal static delegate* unmanaged<IntPtr, byte> class_is_valuetype;
        internal static delegate* unmanaged<IntPtr, IntPtr*, IntPtr> class_get_methods;
        internal static delegate* unmanaged<IntPtr, IntPtr*, IntPtr> class_get_fields;
        internal static delegate* unmanaged<IntPtr, int> class_get_flags;
        internal static delegate* unmanaged<IntPtr, byte*> method_get_name;
        internal static delegate* unmanaged<IntPtr, uint> method_get_param_count;
        internal static delegate* unmanaged<IntPtr, uint*, uint> method_get_flags;
        internal static delegate* unmanaged<IntPtr, IntPtr> method_get_return_type;
        internal static delegate* unmanaged<IntPtr, uint, IntPtr> method_get_param;
        internal static delegate* unmanaged<IntPtr, byte*> field_get_name;
        internal static delegate* unmanaged<IntPtr, nuint> field_get_offset;
        internal static delegate* unmanaged<IntPtr, int> field_get_flags;
        internal static delegate* unmanaged<IntPtr, IntPtr> field_get_type;
        internal static delegate* unmanaged<IntPtr, void*, void> field_static_get_value;
        internal static delegate* unmanaged<IntPtr, void*, void> field_static_set_value;
        internal static delegate* unmanaged<IntPtr, byte*> type_get_name;
        internal static delegate* unmanaged<void*, void> free;
        internal static delegate* unmanaged<IntPtr, IntPtr, void**, IntPtr*, IntPtr> runtime_invoke;
        internal static delegate* unmanaged<IntPtr, IntPtr> object_new;
        internal static delegate* unmanaged<IntPtr, IntPtr> object_get_class;
        internal static delegate* unmanaged<IntPtr, void*> object_unbox;
        internal static delegate* unmanaged<IntPtr, void*, IntPtr> value_box;
        internal static delegate* unmanaged<char*, int, IntPtr> string_new_utf16;
        internal static delegate* unmanaged<IntPtr, char*> string_chars;
        internal static delegate* unmanaged<IntPtr, int> string_length;
        internal static delegate* unmanaged<IntPtr, IntPtr> thread_attach;
        internal static delegate* unmanaged<IntPtr, byte, uint> gchandle_new;
        internal static delegate* unmanaged<uint, IntPtr> gchandle_get_target;
        internal static delegate* unmanaged<uint, void> gchandle_free;
        internal static delegate* unmanaged<byte*, IntPtr> resolve_icall;
        internal static delegate* unmanaged<IntPtr, uint> array_length;
        internal static delegate* unmanaged<IntPtr, nuint, IntPtr> array_new;
        internal static delegate* unmanaged<IntPtr, IntPtr*, IntPtr, void> gc_wbarrier_set_field;
        internal static delegate* unmanaged<IntPtr, byte*, int, void> format_exception;

        static IntPtr Require(string name) {
            var p = Native.Symbol("il2cpp_" + name);
            if (p == IntPtr.Zero) {
                throw new EntryPointNotFoundException("il2cpp_" + name + " is not exported by GameAssembly.dll");
            }
            return p;
        }

        internal static void Init() {
            domain_get = (delegate* unmanaged<IntPtr>)Require("domain_get");
            domain_get_assemblies = (delegate* unmanaged<IntPtr, nuint*, IntPtr*>)Require("domain_get_assemblies");
            assembly_get_image = (delegate* unmanaged<IntPtr, IntPtr>)Require("assembly_get_image");
            image_get_name = (delegate* unmanaged<IntPtr, byte*>)Require("image_get_name");
            image_get_class_count = (delegate* unmanaged<IntPtr, nuint>)Require("image_get_class_count");
            image_get_class = (delegate* unmanaged<IntPtr, nuint, IntPtr>)Require("image_get_class");
            class_from_name = (delegate* unmanaged<IntPtr, byte*, byte*, IntPtr>)Require("class_from_name");
            class_get_method_from_name = (delegate* unmanaged<IntPtr, byte*, int, IntPtr>)Require("class_get_method_from_name");
            class_get_field_from_name = (delegate* unmanaged<IntPtr, byte*, IntPtr>)Require("class_get_field_from_name");
            class_get_name = (delegate* unmanaged<IntPtr, byte*>)Require("class_get_name");
            class_get_namespace = (delegate* unmanaged<IntPtr, byte*>)Require("class_get_namespace");
            class_get_parent = (delegate* unmanaged<IntPtr, IntPtr>)Require("class_get_parent");
            class_get_type = (delegate* unmanaged<IntPtr, IntPtr>)Require("class_get_type");
            class_is_valuetype = (delegate* unmanaged<IntPtr, byte>)Require("class_is_valuetype");
            class_get_methods = (delegate* unmanaged<IntPtr, IntPtr*, IntPtr>)Require("class_get_methods");
            class_get_fields = (delegate* unmanaged<IntPtr, IntPtr*, IntPtr>)Require("class_get_fields");
            class_get_flags = (delegate* unmanaged<IntPtr, int>)Require("class_get_flags");
            method_get_name = (delegate* unmanaged<IntPtr, byte*>)Require("method_get_name");
            method_get_param_count = (delegate* unmanaged<IntPtr, uint>)Require("method_get_param_count");
            method_get_flags = (delegate* unmanaged<IntPtr, uint*, uint>)Require("method_get_flags");
            method_get_return_type = (delegate* unmanaged<IntPtr, IntPtr>)Require("method_get_return_type");
            method_get_param = (delegate* unmanaged<IntPtr, uint, IntPtr>)Require("method_get_param");
            field_get_name = (delegate* unmanaged<IntPtr, byte*>)Require("field_get_name");
            field_get_offset = (delegate* unmanaged<IntPtr, nuint>)Require("field_get_offset");
            field_get_flags = (delegate* unmanaged<IntPtr, int>)Require("field_get_flags");
            field_get_type = (delegate* unmanaged<IntPtr, IntPtr>)Require("field_get_type");
            field_static_get_value = (delegate* unmanaged<IntPtr, void*, void>)Require("field_static_get_value");
            field_static_set_value = (delegate* unmanaged<IntPtr, void*, void>)Require("field_static_set_value");
            type_get_name = (delegate* unmanaged<IntPtr, byte*>)Require("type_get_name");
            free = (delegate* unmanaged<void*, void>)Require("free");
            runtime_invoke = (delegate* unmanaged<IntPtr, IntPtr, void**, IntPtr*, IntPtr>)Require("runtime_invoke");
            object_new = (delegate* unmanaged<IntPtr, IntPtr>)Require("object_new");
            object_get_class = (delegate* unmanaged<IntPtr, IntPtr>)Require("object_get_class");
            object_unbox = (delegate* unmanaged<IntPtr, void*>)Require("object_unbox");
            value_box = (delegate* unmanaged<IntPtr, void*, IntPtr>)Require("value_box");
            string_new_utf16 = (delegate* unmanaged<char*, int, IntPtr>)Require("string_new_utf16");
            string_chars = (delegate* unmanaged<IntPtr, char*>)Require("string_chars");
            string_length = (delegate* unmanaged<IntPtr, int>)Require("string_length");
            thread_attach = (delegate* unmanaged<IntPtr, IntPtr>)Require("thread_attach");
            gchandle_new = (delegate* unmanaged<IntPtr, byte, uint>)Require("gchandle_new");
            gchandle_get_target = (delegate* unmanaged<uint, IntPtr>)Require("gchandle_get_target");
            gchandle_free = (delegate* unmanaged<uint, void>)Require("gchandle_free");
            resolve_icall = (delegate* unmanaged<byte*, IntPtr>)Require("resolve_icall");
            array_length = (delegate* unmanaged<IntPtr, uint>)Require("array_length");
            array_new = (delegate* unmanaged<IntPtr, nuint, IntPtr>)Require("array_new");
            gc_wbarrier_set_field = (delegate* unmanaged<IntPtr, IntPtr*, IntPtr, void>)Native.Symbol("il2cpp_gc_wbarrier_set_field");
            format_exception = (delegate* unmanaged<IntPtr, byte*, int, void>)Native.Symbol("il2cpp_format_exception");
        }

        internal static string Text(byte* p) => p == null ? "" : Marshal.PtrToStringUTF8((IntPtr)p);

        internal static byte[] CString(string s) => Encoding.UTF8.GetBytes((s ?? "") + "\0");

        internal static string TypeName(IntPtr type) {
            if (type == IntPtr.Zero) {
                return "";
            }
            var raw = type_get_name(type);
            var name = Text(raw);
            if (raw != null) {
                free(raw);
            }
            return name;
        }

        public static IntPtr Symbol(string name) => Native.Symbol(name);

        public static void AttachThread() => thread_attach(domain_get());

        public static IEnumerable<IntPtr> Images() {
            nuint count = 0;
            var assemblies = domain_get_assemblies(domain_get(), &count);
            var list = new List<IntPtr>();
            for (nuint i = 0; i < count; i++) {
                var image = assembly_get_image(assemblies[i]);
                if (image != IntPtr.Zero) {
                    list.Add(image);
                }
            }
            return list;
        }

        public static string ImageName(IntPtr image) => Text(image_get_name(image));

        public static IEnumerable<Il2CppClass> Classes(IntPtr image) {
            var count = image_get_class_count(image);
            var list = new List<Il2CppClass>();
            for (nuint i = 0; i < count; i++) {
                var c = image_get_class(image, i);
                if (c != IntPtr.Zero) {
                    list.Add(new Il2CppClass(c));
                }
            }
            return list;
        }

        public static Il2CppClass Class(string ns, string name) {
            var n = CString(ns);
            var c = CString(name);
            nuint count = 0;
            var assemblies = domain_get_assemblies(domain_get(), &count);
            fixed (byte* np = n)
            fixed (byte* cp = c) {
                for (nuint i = 0; i < count; i++) {
                    var klass = class_from_name(assembly_get_image(assemblies[i]), np, cp);
                    if (klass != IntPtr.Zero) {
                        return new Il2CppClass(klass);
                    }
                }
            }
            return default;
        }

        public static Il2CppClass Class(string fullName) {
            var dot = fullName.LastIndexOf('.');
            return dot < 0 ? Class("", fullName) : Class(fullName.Substring(0, dot), fullName.Substring(dot + 1));
        }

        public static Il2CppMethod Method(string ns, string cls, string name, int args = -1) {
            var c = Class(ns, cls);
            return c.IsNull ? default : c.Method(name, args);
        }

        public static Il2CppObject String(string value) {
            if (value == null) {
                return default;
            }
            fixed (char* p = value) {
                return new Il2CppObject(string_new_utf16(p, value.Length));
            }
        }

        public static string ReadString(IntPtr value) {
            if (value == IntPtr.Zero) {
                return null;
            }
            return new string(string_chars(value), 0, string_length(value));
        }

        public static IntPtr Icall(string signature) {
            var s = CString(signature);
            fixed (byte* p = s) {
                return resolve_icall(p);
            }
        }

        internal static void SetReference(IntPtr owner, IntPtr* slot, IntPtr value) {
            if (gc_wbarrier_set_field != null) {
                gc_wbarrier_set_field(owner, slot, value);
            } else {
                *slot = value;
            }
        }

        static int Size(Type type) {
            if (type == typeof(bool) || type == typeof(byte) || type == typeof(sbyte)) {
                return 1;
            }
            if (type == typeof(char) || type == typeof(short) || type == typeof(ushort)) {
                return 2;
            }
            return Marshal.SizeOf(type);
        }

        static void Store(object value, IntPtr dest) {
            switch (value) {
                case bool b: *(byte*)dest = b ? (byte)1 : (byte)0; break;
                case byte v: *(byte*)dest = v; break;
                case sbyte v: *(sbyte*)dest = v; break;
                case char v: *(char*)dest = v; break;
                case short v: *(short*)dest = v; break;
                case ushort v: *(ushort*)dest = v; break;
                case int v: *(int*)dest = v; break;
                case uint v: *(uint*)dest = v; break;
                case long v: *(long*)dest = v; break;
                case ulong v: *(ulong*)dest = v; break;
                case float v: *(float*)dest = v; break;
                case double v: *(double*)dest = v; break;
                default: Marshal.StructureToPtr(value, dest, false); break;
            }
        }

        internal static IntPtr Invoke(IntPtr method, IntPtr instance, object[] args) {
            args ??= Array.Empty<object>();
            var temps = new List<IntPtr>();
            var parameters = stackalloc void*[Math.Max(args.Length, 1)];
            try {
                for (var i = 0; i < args.Length; i++) {
                    var a = args[i];
                    if (a is Enum e) {
                        a = Convert.ChangeType(e, Enum.GetUnderlyingType(e.GetType()));
                    }
                    switch (a) {
                        case null: parameters[i] = null; break;
                        case Il2CppObject o: parameters[i] = (void*)o.Pointer; break;
                        case Il2CppArray arr: parameters[i] = (void*)arr.Pointer; break;
                        case string s: parameters[i] = (void*)String(s).Pointer; break;
                        case IntPtr p: parameters[i] = (void*)p; break;
                        default:
                            var size = Size(a.GetType());
                            var mem = Marshal.AllocHGlobal(Math.Max(size, 8));
                            temps.Add(mem);
                            Store(a, mem);
                            parameters[i] = (void*)mem;
                            break;
                    }
                }
                IntPtr exception = IntPtr.Zero;
                var result = runtime_invoke(method, instance, args.Length == 0 ? null : parameters, &exception);
                if (exception != IntPtr.Zero) {
                    throw new Il2CppException(new Il2CppObject(exception));
                }
                return result;
            } finally {
                foreach (var t in temps) {
                    Marshal.FreeHGlobal(t);
                }
            }
        }

        internal static string Describe(IntPtr exception) {
            if (format_exception != null) {
                var buffer = stackalloc byte[4096];
                buffer[0] = 0;
                format_exception(exception, buffer, 4096);
                var text = Text(buffer);
                if (!string.IsNullOrEmpty(text)) {
                    return text;
                }
            }
            var obj = new Il2CppObject(exception);
            var field = obj.Class.Field("_message");
            var message = field.IsNull ? null : ReadString(field.GetObject(obj).Pointer);
            return obj.Class.FullName + (message == null ? "" : ": " + message);
        }
    }

    public sealed class Il2CppException : Exception {
        public Il2CppObject Exception { get; }

        public Il2CppException(Il2CppObject exception) : base(Il2Cpp.Describe(exception.Pointer)) {
            Exception = exception;
        }
    }

    public sealed unsafe class Il2CppHandle : IDisposable {
        uint handle;

        internal Il2CppHandle(IntPtr target) {
            handle = Il2Cpp.gchandle_new(target, 0);
        }

        public Il2CppObject Target => handle == 0 ? default : new Il2CppObject(Il2Cpp.gchandle_get_target(handle));

        public void Dispose() {
            if (handle != 0) {
                Il2Cpp.gchandle_free(handle);
                handle = 0;
            }
        }
    }

    public readonly unsafe struct Il2CppClass : IEquatable<Il2CppClass> {
        public readonly IntPtr Pointer;

        public Il2CppClass(IntPtr pointer) {
            Pointer = pointer;
        }

        public bool IsNull => Pointer == IntPtr.Zero;
        public string Name => Il2Cpp.Text(Il2Cpp.class_get_name(Pointer));
        public string Namespace => Il2Cpp.Text(Il2Cpp.class_get_namespace(Pointer));
        public string FullName => Il2Cpp.TypeName(Il2Cpp.class_get_type(Pointer));
        public bool IsValueType => Il2Cpp.class_is_valuetype(Pointer) != 0;
        public int Flags => Il2Cpp.class_get_flags(Pointer);
        public IntPtr Type => Il2Cpp.class_get_type(Pointer);

        public Il2CppClass Parent => new Il2CppClass(Il2Cpp.class_get_parent(Pointer));

        public Il2CppMethod Method(string name, int args = -1) {
            if (IsNull) {
                return default;
            }
            var n = Il2Cpp.CString(name);
            fixed (byte* p = n) {
                return new Il2CppMethod(Il2Cpp.class_get_method_from_name(Pointer, p, args));
            }
        }

        public Il2CppField Field(string name) {
            if (IsNull) {
                return default;
            }
            var n = Il2Cpp.CString(name);
            for (var c = this; !c.IsNull; c = c.Parent) {
                fixed (byte* p = n) {
                    var f = Il2Cpp.class_get_field_from_name(c.Pointer, p);
                    if (f != IntPtr.Zero) {
                        return new Il2CppField(f);
                    }
                }
            }
            return default;
        }

        public IEnumerable<Il2CppMethod> Methods {
            get {
                var list = new List<Il2CppMethod>();
                IntPtr iter = IntPtr.Zero;
                IntPtr m;
                while ((m = Il2Cpp.class_get_methods(Pointer, &iter)) != IntPtr.Zero) {
                    list.Add(new Il2CppMethod(m));
                }
                return list;
            }
        }

        public IEnumerable<Il2CppField> Fields {
            get {
                var list = new List<Il2CppField>();
                IntPtr iter = IntPtr.Zero;
                IntPtr f;
                while ((f = Il2Cpp.class_get_fields(Pointer, &iter)) != IntPtr.Zero) {
                    list.Add(new Il2CppField(f));
                }
                return list;
            }
        }

        public Il2CppObject New() => new Il2CppObject(Il2Cpp.object_new(Pointer));

        public Il2CppArray NewArray(int length) => new Il2CppArray(Il2Cpp.array_new(Pointer, (nuint)length));

        public Il2CppObject Box<T>(T value) where T : unmanaged => new Il2CppObject(Il2Cpp.value_box(Pointer, &value));

        public Il2CppObject Call(string method, params object[] args) => Method(method, args?.Length ?? 0).Invoke(default, args);

        public T Call<T>(string method, params object[] args) where T : unmanaged => Method(method, args?.Length ?? 0).Invoke<T>(default, args);

        public T GetStatic<T>(string field) where T : unmanaged => Require(field).GetStatic<T>();

        public void SetStatic<T>(string field, T value) where T : unmanaged => Require(field).SetStatic(value);

        public Il2CppObject GetStaticObject(string field) => Require(field).GetStaticObject();

        Il2CppField Require(string field) {
            var f = Field(field);
            if (f.IsNull) {
                throw new MissingFieldException(FullName, field);
            }
            return f;
        }

        public bool Equals(Il2CppClass other) => Pointer == other.Pointer;
        public override bool Equals(object obj) => obj is Il2CppClass c && Equals(c);
        public override int GetHashCode() => Pointer.GetHashCode();
        public override string ToString() => IsNull ? "null" : FullName;
        public static bool operator ==(Il2CppClass a, Il2CppClass b) => a.Pointer == b.Pointer;
        public static bool operator !=(Il2CppClass a, Il2CppClass b) => a.Pointer != b.Pointer;
    }

    public readonly unsafe struct Il2CppMethod : IEquatable<Il2CppMethod> {
        public readonly IntPtr Pointer;

        public Il2CppMethod(IntPtr pointer) {
            Pointer = pointer;
        }

        public bool IsNull => Pointer == IntPtr.Zero;
        public IntPtr Address => IsNull ? IntPtr.Zero : *(IntPtr*)Pointer;
        public string Name => Il2Cpp.Text(Il2Cpp.method_get_name(Pointer));
        public int ParamCount => (int)Il2Cpp.method_get_param_count(Pointer);
        public string ReturnType => Il2Cpp.TypeName(Il2Cpp.method_get_return_type(Pointer));

        public uint Flags {
            get {
                uint impl = 0;
                return Il2Cpp.method_get_flags(Pointer, &impl);
            }
        }

        public bool IsStatic => (Flags & Il2Cpp.MethodStatic) != 0;

        public string[] ParamTypes {
            get {
                var n = ParamCount;
                var types = new string[n];
                for (var i = 0; i < n; i++) {
                    types[i] = Il2Cpp.TypeName(Il2Cpp.method_get_param(Pointer, (uint)i));
                }
                return types;
            }
        }

        public Il2CppObject Invoke(Il2CppObject instance, params object[] args) {
            if (IsNull) {
                throw new MissingMethodException("IL2CPP method not found");
            }
            return new Il2CppObject(Il2Cpp.Invoke(Pointer, instance.Pointer, args));
        }

        public T Invoke<T>(Il2CppObject instance, params object[] args) where T : unmanaged => Invoke(instance, args).Unbox<T>();

        public bool Equals(Il2CppMethod other) => Pointer == other.Pointer;
        public override bool Equals(object obj) => obj is Il2CppMethod m && Equals(m);
        public override int GetHashCode() => Pointer.GetHashCode();
        public override string ToString() => IsNull ? "null" : Name;
        public static bool operator ==(Il2CppMethod a, Il2CppMethod b) => a.Pointer == b.Pointer;
        public static bool operator !=(Il2CppMethod a, Il2CppMethod b) => a.Pointer != b.Pointer;
    }

    public readonly unsafe struct Il2CppField : IEquatable<Il2CppField> {
        public readonly IntPtr Pointer;

        public Il2CppField(IntPtr pointer) {
            Pointer = pointer;
        }

        public bool IsNull => Pointer == IntPtr.Zero;
        public string Name => Il2Cpp.Text(Il2Cpp.field_get_name(Pointer));
        public int Offset => (int)Il2Cpp.field_get_offset(Pointer);
        public int Flags => Il2Cpp.field_get_flags(Pointer);
        public bool IsStatic => (Flags & Il2Cpp.FieldStatic) != 0;
        public string TypeName => Il2Cpp.TypeName(Il2Cpp.field_get_type(Pointer));

        byte* Address(Il2CppObject obj) {
            if (obj.IsNull) {
                throw new NullReferenceException("IL2CPP object is null");
            }
            return (byte*)obj.Pointer + Offset;
        }

        public T Get<T>(Il2CppObject obj) where T : unmanaged => *(T*)Address(obj);

        public void Set<T>(Il2CppObject obj, T value) where T : unmanaged => *(T*)Address(obj) = value;

        public Il2CppObject GetObject(Il2CppObject obj) => new Il2CppObject(*(IntPtr*)Address(obj));

        public void SetObject(Il2CppObject obj, Il2CppObject value) => Il2Cpp.SetReference(obj.Pointer, (IntPtr*)Address(obj), value.Pointer);

        public T GetStatic<T>() where T : unmanaged {
            T value = default;
            Il2Cpp.field_static_get_value(Pointer, &value);
            return value;
        }

        public void SetStatic<T>(T value) where T : unmanaged => Il2Cpp.field_static_set_value(Pointer, &value);

        public Il2CppObject GetStaticObject() => new Il2CppObject(GetStatic<IntPtr>());

        public void SetStaticObject(Il2CppObject value) => SetStatic(value.Pointer);

        public bool Equals(Il2CppField other) => Pointer == other.Pointer;
        public override bool Equals(object obj) => obj is Il2CppField f && Equals(f);
        public override int GetHashCode() => Pointer.GetHashCode();
        public override string ToString() => IsNull ? "null" : Name;
        public static bool operator ==(Il2CppField a, Il2CppField b) => a.Pointer == b.Pointer;
        public static bool operator !=(Il2CppField a, Il2CppField b) => a.Pointer != b.Pointer;
    }

    public readonly unsafe struct Il2CppObject : IEquatable<Il2CppObject> {
        public readonly IntPtr Pointer;

        public Il2CppObject(IntPtr pointer) {
            Pointer = pointer;
        }

        public bool IsNull => Pointer == IntPtr.Zero;
        public Il2CppClass Class => IsNull ? default : new Il2CppClass(Il2Cpp.object_get_class(Pointer));

        Il2CppField Require(string field) {
            var f = Class.Field(field);
            if (f.IsNull) {
                throw new MissingFieldException(Class.FullName, field);
            }
            return f;
        }

        public T Get<T>(string field) where T : unmanaged => Require(field).Get<T>(this);
        public void Set<T>(string field, T value) where T : unmanaged => Require(field).Set(this, value);
        public Il2CppObject GetObject(string field) => Require(field).GetObject(this);
        public void SetObject(string field, Il2CppObject value) => Require(field).SetObject(this, value);
        public string GetString(string field) => GetObject(field).AsString();

        public Il2CppObject Call(string method, params object[] args) {
            var m = Class.Method(method, args?.Length ?? 0);
            if (m.IsNull) {
                throw new MissingMethodException(Class.FullName, method);
            }
            return m.Invoke(this, args);
        }

        public T Call<T>(string method, params object[] args) where T : unmanaged => Call(method, args).Unbox<T>();

        public T Unbox<T>() where T : unmanaged => IsNull ? default : *(T*)Il2Cpp.object_unbox(Pointer);

        public string AsString() => Il2Cpp.ReadString(Pointer);

        public Il2CppArray AsArray() => new Il2CppArray(Pointer);

        public Il2CppHandle Pin() => new Il2CppHandle(Pointer);

        public static implicit operator IntPtr(Il2CppObject o) => o.Pointer;

        public bool Equals(Il2CppObject other) => Pointer == other.Pointer;
        public override bool Equals(object obj) => obj is Il2CppObject o && Equals(o);
        public override int GetHashCode() => Pointer.GetHashCode();
        public override string ToString() => IsNull ? "null" : Class.FullName + "@" + Pointer.ToString("x");
        public static bool operator ==(Il2CppObject a, Il2CppObject b) => a.Pointer == b.Pointer;
        public static bool operator !=(Il2CppObject a, Il2CppObject b) => a.Pointer != b.Pointer;
    }

    public readonly unsafe struct Il2CppArray {
        public readonly IntPtr Pointer;

        public Il2CppArray(IntPtr pointer) {
            Pointer = pointer;
        }

        public bool IsNull => Pointer == IntPtr.Zero;
        public int Length => IsNull ? 0 : (int)Il2Cpp.array_length(Pointer);
        byte* Data => (byte*)Pointer + Il2Cpp.ArrayDataOffset;

        void Check(int index) {
            if ((uint)index >= (uint)Length) {
                throw new IndexOutOfRangeException();
            }
        }

        public T Get<T>(int index) where T : unmanaged {
            Check(index);
            return ((T*)Data)[index];
        }

        public void Set<T>(int index, T value) where T : unmanaged {
            Check(index);
            ((T*)Data)[index] = value;
        }

        public Il2CppObject GetObject(int index) => new Il2CppObject(Get<IntPtr>(index));

        public void SetObject(int index, Il2CppObject value) {
            Check(index);
            Il2Cpp.SetReference(Pointer, (IntPtr*)Data + index, value.Pointer);
        }

        public Span<T> AsSpan<T>() where T : unmanaged => IsNull ? Span<T>.Empty : new Span<T>(Data, Length);

        public static implicit operator IntPtr(Il2CppArray a) => a.Pointer;
    }
}
