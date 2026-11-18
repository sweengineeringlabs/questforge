@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0publish-demo.ps1" %*
