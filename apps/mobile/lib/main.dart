import 'package:flutter/material.dart';
import 'stego_ffi.dart';

void main() {
  runApp(const OpenStegoApp());
}

class OpenStegoApp extends StatelessWidget {
  const OpenStegoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Stego',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF0F6A5C)),
        useMaterial3: true,
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

  @override
  Widget build(BuildContext context) {
    final pages = [
      const HidePage(),
      const ExtractPage(),
      AboutPage(status: StegoFfi.statusMessage()),
    ];
    return Scaffold(
      appBar: AppBar(title: const Text('Open Stego Mobile')),
      body: pages[_tab],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.lock), label: 'Hide'),
          NavigationDestination(icon: Icon(Icons.lock_open), label: 'Extract'),
          NavigationDestination(icon: Icon(Icons.info_outline), label: 'About'),
        ],
      ),
    );
  }
}

class HidePage extends StatefulWidget {
  const HidePage({super.key});

  @override
  State<HidePage> createState() => _HidePageState();
}

class _HidePageState extends State<HidePage> {
  final _cover = TextEditingController();
  final _payload = TextEditingController();
  final _output = TextEditingController();
  final _password = TextEditingController();
  String _status = 'Enter absolute paths, then Hide. Same format as desktop/CLI.';

  Future<void> _run() async {
    setState(() => _status = 'Working…');
    try {
      final ffi = StegoFfi.load();
      final plan = ffi.plan(_cover.text.trim());
      final result = ffi.hide(
        coverPath: _cover.text.trim(),
        payloadPath: _payload.text.trim(),
        outputPath: _output.text.trim(),
        password: _password.text,
      );
      setState(() {
        _status =
            'plan=${plan['method']} cap=${plan['capacity_bytes']}\nresult=$result';
      });
    } catch (e) {
      setState(() => _status = 'Error: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        TextField(controller: _cover, decoration: const InputDecoration(labelText: 'Cover path')),
        TextField(controller: _payload, decoration: const InputDecoration(labelText: 'Payload path')),
        TextField(controller: _output, decoration: const InputDecoration(labelText: 'Output path')),
        TextField(
          controller: _password,
          obscureText: true,
          decoration: const InputDecoration(labelText: 'Password'),
        ),
        const SizedBox(height: 12),
        FilledButton(onPressed: _run, child: const Text('Hide')),
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
  final _stego = TextEditingController();
  final _output = TextEditingController();
  final _password = TextEditingController();
  String _status = 'Extract → Save. Optional Run only after explicit confirms (desktop model).';

  Future<void> _run() async {
    setState(() => _status = 'Working…');
    try {
      final ffi = StegoFfi.load();
      final result = ffi.extract(
        stegoPath: _stego.text.trim(),
        outputPath: _output.text.trim().isEmpty ? null : _output.text.trim(),
        password: _password.text,
      );
      setState(() => _status = '$result');
      if (result['kind'] == 'executable' && result['output'] != null) {
        final ok = await showDialog<bool>(
          context: context,
          builder: (ctx) => AlertDialog(
            title: const Text('Run recovered file?'),
            content: const Text(
              'Only if you trust the source (educational demo). '
              'Opening the cover in Photos/Acrobat does not run this.',
            ),
            actions: [
              TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
              FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('I understand')),
            ],
          ),
        );
        if (ok == true && mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text('Mobile Run is left to the OS share sheet / file opener — not auto-started.'),
            ),
          );
        }
      }
    } catch (e) {
      setState(() => _status = 'Error: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        TextField(controller: _stego, decoration: const InputDecoration(labelText: 'Stego path')),
        TextField(controller: _output, decoration: const InputDecoration(labelText: 'Output path (files)')),
        TextField(
          controller: _password,
          obscureText: true,
          decoration: const InputDecoration(labelText: 'Password'),
        ),
        const SizedBox(height: 12),
        FilledButton(onPressed: _run, child: const Text('Extract')),
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
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Open Stego Mobile', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 12),
          const Text(
            'Same encrypted format as desktop/CLI. Hide/Extract use stego_ffi → stego-core. '
            'No silent auto-run when opening cover files in the gallery.',
          ),
          const SizedBox(height: 16),
          Text('Bridge: $status'),
        ],
      ),
    );
  }
}
