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

        public Logger(string source) {
            Source = source;
        }

        public void Write(LogLevel level, string message) {
            try {
                Native.Log((int)level, Source, message ?? "");
            } catch (Exception) {
                UnityEngine.Debug.Log($"[{level}] [{Source}] {message}");
            }
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
