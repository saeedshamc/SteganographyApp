/// Dart FFI bindings for `stego_ffi` (cdylib from crates/stego-ffi).
///
/// Build the native library first:
///   cargo build -p stego-ffi --release
/// Then point [DynamicLibrary.open] at the platform path (see apps/mobile/rust/README.md).
library;

import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

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
  StegoFfi._(this._lib)
      : _version = _lib.lookupFunction<_StegoVersionC, _StegoVersionDart>('stego_version'),
        _free = _lib.lookupFunction<_StegoFreeC, _StegoFreeDart>('stego_string_free'),
        _plan = _lib.lookupFunction<_StegoPlanC, _StegoPlanDart>('stego_plan'),
        _hide = _lib.lookupFunction<_StegoHideC, _StegoHideDart>('stego_hide'),
        _extract =
            _lib.lookupFunction<_StegoExtractC, _StegoExtractDart>('stego_extract');

  final DynamicLibrary _lib;
  final _StegoVersionDart _version;
  final _StegoFreeDart _free;
  final _StegoPlanDart _plan;
  final _StegoHideDart _hide;
  final _StegoExtractDart _extract;

  static StegoFfi? _instance;

  static String statusMessage() {
    try {
      return 'native ${load().version()}';
    } catch (e) {
      return 'stub — native lib not loaded ($e)';
    }
  }

  static StegoFfi load([String? path]) {
    if (_instance != null) return _instance!;
    final lib = DynamicLibrary.open(path ?? _defaultLibPath());
    _instance = StegoFfi._(lib);
    return _instance!;
  }

  static String _defaultLibPath() {
    if (Platform.isWindows) {
      return 'stego_ffi.dll';
    }
    if (Platform.isMacOS) {
      return 'libstego_ffi.dylib';
    }
    return 'libstego_ffi.so';
  }

  String version() {
    final p = _version();
    final s = p.toDartString();
    _free(p);
    return s;
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
      final p = payloadPath.toNativeUtf8();
      final o = outputPath.toNativeUtf8();
      final pw = password.toNativeUtf8();
      try {
        return _hide(c, p, o, pw, lsbDepth, out);
      } finally {
        malloc.free(c);
        malloc.free(p);
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
      final o = outputPath?.toNativeUtf8() ?? nullptr;
      final pw = password.toNativeUtf8();
      try {
        return _extract(s, o.cast(), pw, lsbDepth, out);
      } finally {
        malloc.free(s);
        if (outputPath != null) malloc.free(o);
        malloc.free(pw);
      }
    });
  }

  Map<String, dynamic> _callJson(int Function(Pointer<Pointer<Utf8>>) body) {
    final out = calloc<Pointer<Utf8>>();
    try {
      body(out);
      final ptr = out.value;
      if (ptr == nullptr) {
        return {'ok': false, 'error': 'null result'};
      }
      final json = ptr.toDartString();
      _free(ptr);
      return jsonDecode(json) as Map<String, dynamic>;
    } finally {
      calloc.free(out);
    }
  }
}
