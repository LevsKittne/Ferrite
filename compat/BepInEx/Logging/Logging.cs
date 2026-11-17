using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;

namespace BepInEx.Logging {
    [Flags]
    public enum LogLevel {
        None = 0,
        Fatal = 1,
        Error = 2,
        Warning = 4,
        Message = 8,
        Info = 16,
        Debug = 32,
        All = Fatal | Error | Warning | Message | Info | Debug,
    }

    public static class LogLevelExtensions {
        public static LogLevel GetHighestLevel(this LogLevel levels) {
            foreach (var level in new[] { LogLevel.Fatal, LogLevel.Error, LogLevel.Warning, LogLevel.Message, LogLevel.Info, LogLevel.Debug }) {
                if ((levels & level) != LogLevel.None) {
                    return level;
                }
            }
            return LogLevel.None;
        }

        public static ConsoleColor GetConsoleColor(this LogLevel level) {
            level = level.GetHighestLevel();
            switch (level) {
                case LogLevel.Fatal:
                    return ConsoleColor.Red;
                case LogLevel.Error:
                    return ConsoleColor.DarkRed;
                case LogLevel.Warning:
                    return ConsoleColor.Yellow;
                case LogLevel.Message:
                    return ConsoleColor.White;
                case LogLevel.Debug:
                    return ConsoleColor.DarkGray;
                default:
                    return ConsoleColor.Gray;
            }
        }
    }

    public class LogEventArgs : EventArgs {
        public LogEventArgs(object data, LogLevel level, ILogSource source) {
            Data = data;
            Level = level;
            Source = source;
        }

        public object Data { get; protected set; }
        public LogLevel Level { get; protected set; }
        public ILogSource Source { get; protected set; }

        public override string ToString() => $"[{Level,-7}:{Source?.SourceName,10}] {Data}";

        public string ToStringLine() => ToString() + Environment.NewLine;
    }

    public interface ILogSource : IDisposable {
        string SourceName { get; }

        event EventHandler<LogEventArgs> LogEvent;
    }

    public interface ILogListener : IDisposable {
        void LogEvent(object sender, LogEventArgs eventArgs);
    }

    public class ManualLogSource : ILogSource {
        public ManualLogSource(string sourceName) {
            SourceName = sourceName;
        }

        public string SourceName { get; }

        public event EventHandler<LogEventArgs> LogEvent;

        public void Log(LogLevel level, object data) => LogEvent?.Invoke(this, new LogEventArgs(data, level, this));

        public void LogFatal(object data) => Log(LogLevel.Fatal, data);
        public void LogError(object data) => Log(LogLevel.Error, data);
        public void LogWarning(object data) => Log(LogLevel.Warning, data);
        public void LogMessage(object data) => Log(LogLevel.Message, data);
        public void LogInfo(object data) => Log(LogLevel.Info, data);
        public void LogDebug(object data) => Log(LogLevel.Debug, data);

        public void Dispose() {
            Logger.Sources.Remove(this);
        }
    }

    public static class Logger {
        static readonly List<ILogListener> listeners = new List<ILogListener> { new FerriteLogListener() };
        static readonly SourceCollection sources = new SourceCollection();
        static readonly ManualLogSource internalSource = CreateLogSource("BepInEx");

        public static ICollection<ILogListener> Listeners => listeners;
        public static ICollection<ILogSource> Sources => sources;

        public static ManualLogSource CreateLogSource(string sourceName) {
            var source = new ManualLogSource(sourceName);
            Sources.Add(source);
            return source;
        }

        internal static void Log(LogLevel level, object data) => internalSource.Log(level, data);

        internal static void InternalLogEvent(object sender, LogEventArgs eventArgs) {
            ILogListener[] snapshot;
            lock (listeners) {
                snapshot = listeners.ToArray();
            }
            foreach (var listener in snapshot) {
                try {
                    listener?.LogEvent(sender, eventArgs);
                } catch (Exception) {
                }
            }
        }

        sealed class SourceCollection : ICollection<ILogSource> {
            readonly List<ILogSource> items = new List<ILogSource>();

            public int Count {
                get {
                    lock (items) {
                        return items.Count;
                    }
                }
            }

            public bool IsReadOnly => false;

            public void Add(ILogSource item) {
                if (item == null) {
                    throw new ArgumentNullException(nameof(item));
                }
                lock (items) {
                    items.Add(item);
                }
                item.LogEvent += InternalLogEvent;
            }

            public bool Remove(ILogSource item) {
                bool removed;
                lock (items) {
                    removed = items.Remove(item);
                }
                if (removed) {
                    item.LogEvent -= InternalLogEvent;
                }
                return removed;
            }

            public void Clear() {
                foreach (var item in this.ToArray()) {
                    Remove(item);
                }
            }

            public bool Contains(ILogSource item) {
                lock (items) {
                    return items.Contains(item);
                }
            }

            public void CopyTo(ILogSource[] array, int arrayIndex) {
                lock (items) {
                    items.CopyTo(array, arrayIndex);
                }
            }

            public IEnumerator<ILogSource> GetEnumerator() {
                lock (items) {
                    return items.ToList().GetEnumerator();
                }
            }

            IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
        }
    }

    sealed class FerriteLogListener : ILogListener {
        readonly Dictionary<string, Ferrite.Logger> loggers = new Dictionary<string, Ferrite.Logger>();

        public void LogEvent(object sender, LogEventArgs eventArgs) {
            var name = eventArgs.Source?.SourceName ?? "BepInEx";
            Ferrite.Logger logger;
            lock (loggers) {
                if (!loggers.TryGetValue(name, out logger)) {
                    logger = new Ferrite.Logger(name);
                    loggers[name] = logger;
                }
            }
            logger.Write(Map(eventArgs.Level), eventArgs.Data?.ToString() ?? "null");
        }

        static Ferrite.LogLevel Map(LogLevel level) {
            if ((level & (LogLevel.Fatal | LogLevel.Error)) != 0) {
                return Ferrite.LogLevel.Error;
            }
            if ((level & LogLevel.Warning) != 0) {
                return Ferrite.LogLevel.Warning;
            }
            if ((level & (LogLevel.Message | LogLevel.Info)) != 0) {
                return Ferrite.LogLevel.Info;
            }
            return Ferrite.LogLevel.Debug;
        }

        public void Dispose() {
        }
    }
}
