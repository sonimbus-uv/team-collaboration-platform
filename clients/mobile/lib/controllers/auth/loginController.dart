import 'package:flutter/material.dart';

class LoginController extends ChangeNotifier {
  final formKey = GlobalKey<FormState>();
  final emailController = TextEditingController();
  final passwordController = TextEditingController();

  bool obscurePassword = true;
  bool hasError = false;
  bool isLoading = false;

  // Mensaje genérico para cumplir con el criterio de seguridad de RF-AUTH-02
  String errorMessage = 'Correo o contraseña incorrectos';

  void togglePasswordVisibility() {
    obscurePassword = !obscurePassword;
    notifyListeners();
  }

  Future<bool> handleLogin() async {
    if (emailController.text.isEmpty || passwordController.text.isEmpty) {
      hasError = true;
      errorMessage = 'Todos los campos son obligatorios';
      notifyListeners();
      return false;
    }

    isLoading = true;
    hasError = false;
    notifyListeners();

    try {
      // TODO: Implementar consumo real a Auth Service (Rust)
      // Simulación de latencia de red
      await Future.delayed(const Duration(seconds: 2));

      // Simulación de validación exitosa
      isLoading = false;
      notifyListeners();
      return true;
    } catch (e) {
      isLoading = false;
      hasError = true;
      errorMessage = 'Correo o contraseña incorrectos';
      notifyListeners();
      return false;
    }
  }

  @override
  void dispose() {
    emailController.dispose();
    passwordController.dispose();
    super.dispose();
  }
}
