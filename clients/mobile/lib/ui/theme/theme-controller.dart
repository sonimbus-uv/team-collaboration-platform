import 'package:flutter/material.dart';

class ThemeController extends ValueNotifier<ThemeMode> {
  ThemeController() : super(ThemeMode.light);

  bool get isDark => value == ThemeMode.dark;

  void setThemeMode(ThemeMode mode) {
    if (value != mode) {
      value = mode;
    }
  }

  void toggleTheme() {
    value = isDark ? ThemeMode.light : ThemeMode.dark;
  }
}
