import 'package:flutter_test/flutter_test.dart';
import 'package:open_stego_mobile/main.dart';

void main() {
  testWidgets('Open Stego shell shows Hide tab', (WidgetTester tester) async {
    await tester.pumpWidget(const OpenStegoApp());
    expect(find.text('Hide'), findsWidgets);
    expect(find.text('Open Stego'), findsOneWidget);
  });
}
