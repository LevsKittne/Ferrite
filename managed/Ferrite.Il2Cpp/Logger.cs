using System;

namespace Ferrite {
    public enum LogLevel {
        Debug,
        Info,
        Warning,
        Error,
    }

    public sealed class Logger {
        readonly IntPtr handle;

        public string Source { get; }

        internal Logger(IntPtr handle, string source) {
            this.handle = handle;
            Source = source;
        }

        public void Write(LogLevel level, string message) => Native.Log(handle, (int)level, message ?? "");

        public void Debug(string message) => Write(LogLevel.Debug, message);
        public void Info(string message) => Write(LogLevel.Info, message);
        public void Warning(string message) => Write(LogLevel.Warning, message);
        public void Error(string message) => Write(LogLevel.Error, message);
    }
}
