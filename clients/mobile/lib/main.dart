import 'package:flutter/material.dart';

import 'ui/auth/login.dart';
import 'ui/auth/recover-password.dart';
import 'ui/auth/register.dart';
import 'ui/auth/verify-account.dart';
import 'ui/groups/group-settings.dart';
import 'ui/notifications/notifications.dart';
import 'ui/profile/profile.dart';
import 'ui/shell/main-menu.dart';
import 'ui/text/text-channel.dart';
import 'ui/theme/app-theme.dart';
import 'ui/theme/theme-controller.dart';
import 'ui/theme/theme-scope.dart';
import 'ui/voice/voice-channel.dart';

void main() {
  runApp(const SonimbusApp());
}

class SonimbusApp extends StatefulWidget {
  const SonimbusApp({super.key});

  @override
  State<SonimbusApp> createState() => _SonimbusAppState();
}

class _SonimbusAppState extends State<SonimbusApp> {
  final ThemeController _themeController = ThemeController();

  @override
  void dispose() {
    _themeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ThemeScope(
      controller: _themeController,
      child: ValueListenableBuilder<ThemeMode>(
        valueListenable: _themeController,
        builder: (context, currentMode, _) {
          return MaterialApp(
            title: 'Sonimbus',
            debugShowCheckedModeBanner: false,
            theme: AppTheme.light,
            darkTheme: AppTheme.dark,
            themeMode: currentMode,
            initialRoute: '/login',
            routes: {
              '/login': (context) => const LoginScreen(),
              '/register': (context) => const RegisterScreen(),
              '/verify-account': (context) => const VerifyAccountScreen(),
              '/recover-password': (context) => const RecoverPasswordScreen(),
              '/voice-channel': (context) => const VoiceChannelView(),
              '/text-channel': (context) => const TextChannelView(),
              '/main-menu': (context) => const MainShellScreen(),
              '/notifications': (context) => const NotificationsScreen(),
              '/profile': (context) => const ProfileScreen(),
              '/group-settings': (context) => const GroupSettingsScreen(),
            },
          );
        },
      ),
    );
  }
}
