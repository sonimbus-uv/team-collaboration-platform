import 'package:flutter/material.dart';

import 'ui/auth/login.dart';
import 'ui/auth/register.dart';
import 'ui/auth/verify-account.dart';
import 'ui/auth/recover-password.dart';

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
      },
    );
  }
}
