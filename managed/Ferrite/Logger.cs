using System;
using System.IO;
using System.Text;

namespace Ferrite {
    public enum LogLevel {
        Debug,
        Info,
        Warning,
        Error,
    }

    public sealed class Logger {
        public string Source { get; }

        static long seen = -1;
        static string[] recent = new string[0];

        public static string[] Recent {
            get {
                var seq = Native.LogSequence();
                if (seq != seen) {
                    seen = seq;
                    recent = (Native.LogTail() ?? "").Split(new[] { '\n' },StringSplitOptions.RemoveEmptyEntries);
                }
                return recent;
            }
        }

        public Logger(string source) {
            Source = source;
        }

        public void Write(LogLevel level, string message) {
            try {
                Native.Log((int)level, Source, message ?? "");
            } catch (Exception) {
                Fallback(level, message);
            }
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        void Fallback(LogLevel level, string message) {
            UnityEngine.Debug.Log($"[{level}] [{Source}] {message}");
        }

        public void Debug(string message) => Write(LogLevel.Debug, message);
        public void Info(string message) => Write(LogLevel.Info, message);
        public void Warning(string message) => Write(LogLevel.Warning, message);
        public void Error(string message) => Write(LogLevel.Error, message);
    }

    sealed class LoggerWriter : TextWriter {
        readonly Logger logger;
        readonly StringBuilder line = new StringBuilder();

        public LoggerWriter(Logger logger) {
            this.logger = logger;
        }

        public override Encoding Encoding => Encoding.UTF8;

        public override void Write(char value) {
            lock (line) {
                if (value == '\n') {
                    Flush();
                } else if (value != '\r') {
                    line.Append(value);
                }
            }
        }

        public override void Write(string value) {
            if (value == null) {
                return;
            }
            foreach (var c in value) {
                Write(c);
            }
        }

        public override void Flush() {
            lock (line) {
                if (line.Length == 0) {
                    return;
                }
                logger.Info(line.ToString());
                line.Length = 0;
            }
        }
    }
}
