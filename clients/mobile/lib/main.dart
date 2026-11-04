import 'package:flutter/material.dart';

import 'ui/auth/login.dart';
import 'ui/auth/register.dart';
import 'ui/auth/verify-account.dart';
import 'ui/auth/recover-password.dart';
import 'ui/shell/main-menu.dart';
import 'ui/notifications/notifications.dart';
import 'ui/profile/profile.dart';
import 'ui/groups/group-settings.dart';
import 'ui/voice/voice-channel.dart';
import 'ui/text/text-channel.dart'; // <-- Nueva importación

void main() {
  runApp(const SonimbusApp());
}

class SonimbusApp extends StatelessWidget {
  const SonimbusApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Sonimbus',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        scaffoldBackgroundColor: Colors.white,
        useMaterial3: true,
      ),
      initialRoute: '/login',
      routes: {
        '/login': (context) => const LoginScreen(),
        '/register': (context) => const RegisterScreen(),
        '/verify-account': (context) => const VerifyAccountScreen(),
        '/recover-password': (context) => const RecoverPasswordScreen(),
        '/voice-channel': (context) => const VoiceChannelView(),
        '/text-channel': (context) =>
            const TextChannelView(), // <-- Nueva ruta para el canal de texto
        '/main-menu': (context) => const MainShellScreen(),
        '/notifications': (context) => const NotificationsScreen(),
        '/profile': (context) => const ProfileScreen(),
        '/group-settings': (context) => const GroupSettingsScreen(),
      },
    );
  }
}
