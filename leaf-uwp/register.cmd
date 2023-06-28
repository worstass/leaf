cargo build -Z build-std=std,panic_abort --target aarch64-uwp-windows-msvc
copy appx\* ..\target\debug
cd ..\target\debug
powershell -command "Get-AppxPackage LeafVpn | Remove-AppxPackage"
powershell -command "Add-AppxPackage -Register AppxManifest.xml"
cd ..\..\leaf-uwp