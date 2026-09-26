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
      home: Scaffold(
        appBar: AppBar(title: const Text('Open Stego Mobile')),
        body: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Phase F scaffold',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 12),
              const Text(
                'FFI to stego-core will power Hide / Extract with the same '
                'transparent demo flow as desktop (Save, then optional Run with confirm).',
              ),
              const SizedBox(height: 16),
              Text('Bridge status: ${StegoFfi.statusMessage()}'),
            ],
          ),
        ),
      ),
    );
  }
}
