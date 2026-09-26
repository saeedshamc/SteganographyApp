/// Dart FFI bindings for `stego_ffi` (cdylib from crates/stego-ffi).
library;

import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:path/path.dart' as p;

typedef _StegoVersionC = Pointer<Utf8> Function();
typedef _StegoVersionDart = Pointer<Utf8> Function();
typedef _StegoFreeC = Void Function(Pointer<Utf8>);
typedef _StegoFreeDart = void Function(Pointer<Utf8>);
typedef _StegoPlanC = Int32 Function(
  Pointer<Utf8>,
  Int32,
  Pointer<Pointer<Utf8>>,
);
typedef _StegoPlanDart = int Function(
  Pointer<Utf8>,
  int,
  Pointer<Pointer<Utf8>>,
);
typedef _StegoHideC = Int32 Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Int32,
  Pointer<Pointer<Utf8>>,
);
typedef _StegoHideDart = int Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  int,
  Pointer<Pointer<Utf8>>,
);
typedef _StegoExtractC = Int32 Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  Int32,
  Pointer<Pointer<Utf8>>,
);
typedef _StegoExtractDart = int Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Utf8>,
  int,
  Pointer<Pointer<Utf8>>,
);

class StegoFfi {
  StegoFfi._(DynamicLibrary lib)
      : _version =
            lib.lookupFunction<_StegoVersionC, _StegoVersionDart>('stego_version'),
        _free =
            lib.lookupFunction<_StegoFreeC, _StegoFreeDart>('stego_string_free'),
        _plan = lib.lookupFunction<_StegoPlanC, _StegoPlanDart>('stego_plan'),
        _hide = lib.lookupFunction<_StegoHideC, _StegoHideDart>('stego_hide'),
        _extract =
            lib.lookupFunction<_StegoExtractC, _StegoExtractDart>('stego_extract');

  final _StegoVersionDart _version;
  final _StegoFreeDart _free;
  final _StegoPlanDart _plan;
  final _StegoHideDart _hide;
  final _StegoExtractDart _extract;

  static StegoFfi? _instance;
  static String? _loadedFrom;

  static String? get loadedFrom => _loadedFrom;

  static String statusMessage() {
    try {
      final ffi = load();
      return 'native ${ffi.version()} ← ${_loadedFrom ?? "?"}';
    } catch (e) {
      return 'native lib not loaded ($e)';
    }
  }

  static StegoFfi load([String? path]) {
    if (_instance != null) return _instance!;
    final candidates = <String>[
      if (path != null) path,
      ..._candidatePaths(),
    ];
    Object? last;
    for (final c in candidates) {
      try {
        if (!_looksLoadable(c)) continue;
        final lib = DynamicLibrary.open(c);
        _instance = StegoFfi._(lib);
        _loadedFrom = c;
        return _instance!;
      } catch (e) {
        last = e;
      }
    }
    throw StateError(
      'Could not load stego_ffi. Tried:\n${candidates.join("\n")}\nLast error: $last',
    );
  }

  static bool _looksLoadable(String path) {
    // Android package-relative sonames have no path separator.
    if (!path.contains('/') && !path.contains(r'\')) return true;
    return File(path).existsSync();
  }

  static List<String> _candidatePaths() {
    final name = _libFileName();
    final out = <String>[name];
    try {
      final exe = Platform.resolvedExecutable;
      final exeDir = p.dirname(exe);
      out.add(p.join(exeDir, name));
      out.add(p.join(exeDir, 'lib', name));
      // Dev: repo target/release next to apps/mobile
      out.add(p.normalize(p.join(exeDir, '..', '..', '..', '..', 'target', 'release', name)));
      out.add(p.normalize(p.join(Directory.current.path, '..', '..', 'target', 'release', name)));
      out.add(p.join(Directory.current.path, 'native', name));
    } catch (_) {}
    if (Platform.isAndroid) {
      // Bundled via jniLibs as libstego_ffi.so
      out.insert(0, 'libstego_ffi.so');
    }
    return out;
  }

  static String _libFileName() {
    if (Platform.isWindows) return 'stego_ffi.dll';
    if (Platform.isMacOS) return 'libstego_ffi.dylib';
    if (Platform.isIOS) return 'stego_ffi.framework/stego_ffi';
    return 'libstego_ffi.so';
  }

  String version() {
    final ptr = _version();
    try {
      return ptr.toDartString();
    } finally {
      _free(ptr);
    }
  }

  Map<String, dynamic> plan(String coverPath, {int lsbDepth = 1}) {
    return _callJson((out) {
      final c = coverPath.toNativeUtf8();
      try {
        return _plan(c, lsbDepth, out);
      } finally {
        malloc.free(c);
      }
    });
  }

  Map<String, dynamic> hide({
    required String coverPath,
    required String payloadPath,
    required String outputPath,
    required String password,
    int lsbDepth = 1,
  }) {
    return _callJson((out) {
      final c = coverPath.toNativeUtf8();
      final payload = payloadPath.toNativeUtf8();
      final o = outputPath.toNativeUtf8();
      final pw = password.toNativeUtf8();
      try {
        return _hide(c, payload, o, pw, lsbDepth, out);
      } finally {
        malloc.free(c);
        malloc.free(payload);
        malloc.free(o);
        malloc.free(pw);
      }
    });
  }

  Map<String, dynamic> extract({
    required String stegoPath,
    String? outputPath,
    required String password,
    int lsbDepth = 1,
  }) {
    return _callJson((out) {
      final s = stegoPath.toNativeUtf8();
      final o = (outputPath == null || outputPath.isEmpty)
          ? nullptr
          : outputPath.toNativeUtf8();
      final pw = password.toNativeUtf8();
      try {
        return _extract(s, o.cast(), pw, lsbDepth, out);
      } finally {
        malloc.free(s);
        if (o != nullptr) malloc.free(o);
        malloc.free(pw);
      }
    });
  }

  Map<String, dynamic> _callJson(int Function(Pointer<Pointer<Utf8>>) body) {
    final out = calloc<Pointer<Utf8>>();
    try {
      final rc = body(out);
      final ptr = out.value;
      if (ptr == nullptr) {
        return {'ok': false, 'error': 'null result', 'rc': rc};
      }
      final json = ptr.toDartString();
      _free(ptr);
      final map = jsonDecode(json) as Map<String, dynamic>;
      map.putIfAbsent('rc', () => rc);
      return map;
    } finally {
      calloc.free(out);
    }
  }
}
