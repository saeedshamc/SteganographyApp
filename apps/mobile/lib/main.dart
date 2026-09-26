import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:open_filex/open_filex.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

import 'stego_ffi.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const OpenStegoApp());
}

class OpenStegoApp extends StatelessWidget {
  const OpenStegoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Stego',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0F6A5C),
          brightness: Brightness.light,
        ),
        useMaterial3: true,
        fontFamily: Platform.isWindows ? 'Segoe UI' : null,
      ),
      home: const HomeShell(),
    );
  }
}

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _tab = 0;
  late final String _bridgeStatus = StegoFfi.statusMessage();

  @override
  Widget build(BuildContext context) {
    final pages = [
      const HidePage(),
      const ExtractPage(),
      AboutPage(status: _bridgeStatus),
    ];
    return Scaffold(
      appBar: AppBar(
        title: const Text('Open Stego'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 12),
            child: Center(
              child: Text(
                'v0.4.1',
                style: Theme.of(context).textTheme.labelMedium,
              ),
            ),
          ),
        ],
      ),
      body: pages[_tab],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.lock_outline), label: 'Hide'),
          NavigationDestination(icon: Icon(Icons.lock_open), label: 'Extract'),
          NavigationDestination(icon: Icon(Icons.info_outline), label: 'About'),
        ],
      ),
    );
  }
}

Future<String?> _pickFile({List<String>? extensions, String? dialogTitle}) async {
  final result = await FilePicker.platform.pickFiles(
    type: extensions == null ? FileType.any : FileType.custom,
    allowedExtensions: extensions,
    dialogTitle: dialogTitle,
    withData: Platform.isAndroid,
  );
  if (result == null || result.files.isEmpty) return null;
  final f = result.files.single;
  if (f.path != null && f.path!.isNotEmpty) return f.path;
  // Android content URI without a path: materialize into app documents.
  if (f.bytes == null) return null;
  final dir = await getApplicationDocumentsDirectory();
  final name = f.name.isEmpty ? 'picked.bin' : f.name;
  final out = File(p.join(dir.path, 'picked_$name'));
  await out.writeAsBytes(f.bytes!, flush: true);
  return out.path;
}

Future<String?> _pickSavePath({required String fileName, String? dialogTitle}) async {
  // Desktop: let the user choose. Mobile: write under documents.
  if (Platform.isAndroid || Platform.isIOS) {
    final dir = await getApplicationDocumentsDirectory();
    return p.join(dir.path, fileName);
  }
  return FilePicker.platform.saveFile(
    dialogTitle: dialogTitle ?? 'Save output',
    fileName: fileName,
  );
}

class HidePage extends StatefulWidget {
  const HidePage({super.key});

  @override
  State<HidePage> createState() => _HidePageState();
}

class _HidePageState extends State<HidePage> {
  String? _cover;
  String? _payload;
  String? _output;
  final _password = TextEditingController();
  int _lsbDepth = 1;
  String _status = 'Pick a cover and payload, set a password, then Hide.';
  String? _planLine;
  bool _busy = false;

  @override
  void dispose() {
    _password.dispose();
    super.dispose();
  }

  Future<void> _refreshPlan() async {
    if (_cover == null) return;
    try {
      final plan = StegoFfi.load().plan(_cover!, lsbDepth: _lsbDepth);
      if (plan['ok'] == false) {
        setState(() => _planLine = 'Plan error: ${plan['error']}');
        return;
      }
      final cap = plan['capacity_bytes'];
      setState(() {
        _planLine =
            'Method ${plan['method']}'
            '${cap != null ? ' · capacity $cap bytes' : ' · EOF (unbounded)'}'
            '${plan['eof_caveat'] != null ? '\n${plan['eof_caveat']}' : ''}'
            '${plan['jpeg_warning'] != null ? '\n${plan['jpeg_warning']}' : ''}';
      });
    } catch (e) {
      setState(() => _planLine = 'Plan failed: $e');
    }
  }

  Future<void> _run() async {
    if (_cover == null || _payload == null) {
      setState(() => _status = 'Cover and payload are required.');
      return;
    }
    if (_password.text.isEmpty) {
      setState(() => _status = 'Password is required.');
      return;
    }
    setState(() {
      _busy = true;
      _status = 'Working…';
    });
    try {
      var out = _output;
      out ??= await _pickSavePath(
        fileName: '${p.basenameWithoutExtension(_cover!)}_stego${p.extension(_cover!).isEmpty ? '.bin' : p.extension(_cover!)}',
        dialogTitle: 'Save stego output',
      );
      if (out == null || out.isEmpty) {
        setState(() {
          _busy = false;
          _status = 'Save cancelled.';
        });
        return;
      }
      _output = out;
      final ffi = StegoFfi.load();
      await _refreshPlan();
      final result = ffi.hide(
        coverPath: _cover!,
        payloadPath: _payload!,
        outputPath: out,
        password: _password.text,
        lsbDepth: _lsbDepth,
      );
      setState(() {
        _busy = false;
        _status = result['ok'] == true
            ? 'Saved ${result['output']} (${result['bytes']} bytes, .${result['extension']}, kind=${result['kind']})'
            : 'Hide failed: ${result['error']}';
      });
    } catch (e) {
      setState(() {
        _busy = false;
        _status = 'Error: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _PathTile(
          label: 'Cover',
          value: _cover,
          onPick: () async {
            final path = await _pickFile(dialogTitle: 'Choose cover');
            if (path == null) return;
            setState(() => _cover = path);
            await _refreshPlan();
          },
        ),
        _PathTile(
          label: 'Payload',
          value: _payload,
          onPick: () async {
            final path = await _pickFile(dialogTitle: 'Choose payload');
            if (path == null) return;
            setState(() => _payload = path);
          },
        ),
        _PathTile(
          label: 'Output (optional before Hide)',
          value: _output,
          onPick: () async {
            final path = await _pickSavePath(
              fileName: 'stego_out.bin',
              dialogTitle: 'Choose output path',
            );
            if (path == null) return;
            setState(() => _output = path);
          },
        ),
        const SizedBox(height: 8),
        TextField(
          controller: _password,
          obscureText: true,
          decoration: const InputDecoration(
            labelText: 'Password',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 12),
        DropdownButtonFormField<int>(
          initialValue: _lsbDepth,
          decoration: const InputDecoration(
            labelText: 'LSB depth (Method A)',
            border: OutlineInputBorder(),
          ),
          items: const [
            DropdownMenuItem(value: 1, child: Text('1 (default)')),
            DropdownMenuItem(value: 2, child: Text('2 (more capacity, noisier)')),
          ],
          onChanged: (v) async {
            if (v == null) return;
            setState(() => _lsbDepth = v);
            await _refreshPlan();
          },
        ),
        if (_planLine != null) ...[
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Text(_planLine!),
            ),
          ),
        ],
        const SizedBox(height: 16),
        FilledButton(
          onPressed: _busy ? null : _run,
          child: Text(_busy ? 'Working…' : 'Hide'),
        ),
        const SizedBox(height: 16),
        Text(_status),
      ],
    );
  }
}

class ExtractPage extends StatefulWidget {
  const ExtractPage({super.key});

  @override
  State<ExtractPage> createState() => _ExtractPageState();
}

class _ExtractPageState extends State<ExtractPage> {
  String? _stego;
  String? _output;
  final _password = TextEditingController();
  int _lsbDepth = 1;
  String _status =
      'Extract → Save. Optional open/run only after an explicit confirm (educational).';
  bool _busy = false;
  String? _lastSaved;
  bool _lastExecutable = false;

  @override
  void dispose() {
    _password.dispose();
    super.dispose();
  }

  Future<void> _run() async {
    if (_stego == null) {
      setState(() => _status = 'Choose a stego file first.');
      return;
    }
    if (_password.text.isEmpty) {
      setState(() => _status = 'Password is required.');
      return;
    }
    setState(() {
      _busy = true;
      _status = 'Working…';
      _lastSaved = null;
      _lastExecutable = false;
    });
    try {
      final ffi = StegoFfi.load();
      // Probe with a temp output for file payloads; text returns in JSON.
      final suggested = _output ??
          p.join(
            (await getApplicationDocumentsDirectory()).path,
            'recovered.bin',
          );
      final result = ffi.extract(
        stegoPath: _stego!,
        outputPath: suggested,
        password: _password.text,
        lsbDepth: _lsbDepth,
      );
      if (result['ok'] != true) {
        setState(() {
          _busy = false;
          _status = 'Extract failed: ${result['error']}';
        });
        return;
      }
      if (result['kind'] == 'text') {
        setState(() {
          _busy = false;
          _status = 'Recovered text (${result['size']} bytes) via ${result['method']}:\n${result['text']}';
        });
        return;
      }

      var outPath = result['output'] as String? ?? suggested;
      // If user wanted a different save location on desktop, re-pick.
      if (!Platform.isAndroid && !Platform.isIOS) {
        final chosen = await _pickSavePath(
          fileName: (result['filename'] as String?) ?? p.basename(outPath),
          dialogTitle: 'Save recovered file',
        );
        if (chosen != null && chosen.isNotEmpty && chosen != outPath) {
          await File(outPath).copy(chosen);
          outPath = chosen;
        }
      }

      final kind = result['kind'] as String? ?? 'file';
      setState(() {
        _busy = false;
        _lastSaved = outPath;
        _lastExecutable = kind == 'executable';
        _status =
            'Saved $outPath via ${result['method']} [$kind] (${result['size']} bytes)';
      });
    } catch (e) {
      setState(() {
        _busy = false;
        _status = 'Error: $e';
      });
    }
  }

  Future<void> _confirmOpen() async {
    if (_lastSaved == null) return;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(_lastExecutable ? 'Run recovered program?' : 'Open recovered file?'),
        content: Text(
          _lastExecutable
              ? 'You extracted this yourself inside Open Stego.\n\n'
                  'Continue only if you trust the source (educational demo).\n'
                  'Opening a cover in Photos/Acrobat does NOT run this.'
              : 'Open the recovered file with the system handler?',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(_lastExecutable ? 'I understand — open' : 'Open'),
          ),
        ],
      ),
    );
    if (ok != true) return;
    if (!mounted) return;
    if (_lastExecutable) {
      final ok2 = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('Final confirm'),
          content: const Text('Start/open the recovered executable now?'),
          actions: [
            TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
            FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Open')),
          ],
        ),
      );
      if (ok2 != true) return;
      if (!mounted) return;
    }
    final r = await OpenFilex.open(_lastSaved!);
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Open result: ${r.message}')),
    );
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _PathTile(
          label: 'Stego file',
          value: _stego,
          onPick: () async {
            final path = await _pickFile(dialogTitle: 'Choose stego file');
            if (path == null) return;
            setState(() => _stego = path);
          },
        ),
        TextField(
          controller: _password,
          obscureText: true,
          decoration: const InputDecoration(
            labelText: 'Password',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 12),
        DropdownButtonFormField<int>(
          initialValue: _lsbDepth,
          decoration: const InputDecoration(
            labelText: 'LSB depth hint',
            border: OutlineInputBorder(),
          ),
          items: const [
            DropdownMenuItem(value: 1, child: Text('1')),
            DropdownMenuItem(value: 2, child: Text('2')),
          ],
          onChanged: (v) {
            if (v == null) return;
            setState(() => _lsbDepth = v);
          },
        ),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: _busy ? null : _run,
          child: Text(_busy ? 'Working…' : 'Extract'),
        ),
        if (_lastSaved != null) ...[
          const SizedBox(height: 8),
          OutlinedButton(
            onPressed: _confirmOpen,
            child: Text(_lastExecutable ? 'Open / Run with confirm…' : 'Open recovered file…'),
          ),
        ],
        const SizedBox(height: 16),
        Text(_status),
      ],
    );
  }
}

class AboutPage extends StatelessWidget {
  const AboutPage({super.key, required this.status});
  final String status;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Center(
          child: Image.asset(
            'assets/branding/logo.png',
            width: 96,
            height: 96,
            filterQuality: FilterQuality.high,
          ),
        ),
        const SizedBox(height: 16),
        Text('Open Stego Mobile', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 12),
        const Text(
          'Same encrypted on-disk format as the desktop app and stego CLI. '
          'Hide / Extract call stego-ffi → stego-core (Argon2id + AES-GCM, Method A/B).',
        ),
        const SizedBox(height: 12),
        const Text(
          'There is no silent auto-run when you open a cover in the gallery or a PDF viewer. '
          'Recovered executables open only after explicit confirms in this app.',
        ),
        const SizedBox(height: 16),
        Card(
          child: ListTile(
            title: const Text('Native bridge'),
            subtitle: Text(status),
          ),
        ),
        const SizedBox(height: 8),
        const Text('Android build guide: docs/ANDROID.md'),
        const Text('Native lib: scripts/build-mobile-native.ps1 -Android'),
      ],
    );
  }
}

class _PathTile extends StatelessWidget {
  const _PathTile({
    required this.label,
    required this.value,
    required this.onPick,
  });

  final String label;
  final String? value;
  final VoidCallback onPick;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: ListTile(
        title: Text(label),
        subtitle: Text(
          value ?? 'Not selected',
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        trailing: FilledButton.tonal(
          onPressed: onPick,
          child: const Text('Browse'),
        ),
      ),
    );
  }
}
