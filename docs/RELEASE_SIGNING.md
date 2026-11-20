# Firma y entrega de candidatos

Preparación 2026-11-16. El piloto 0.2.0 usa paquetes locales sin firma. No se generaron certificados ni se firmó/publicó una release; DevOps aprovisionará las credenciales de publicación.

## Windows

`scripts/sign-windows-release.ps1` usa SignTool del SDK de Windows y un certificado organizacional con clave privada disponible en el almacén My de CurrentUser o LocalMachine. No recibe contraseñas PFX ni exporta claves. Trabaja sobre copias en un directorio nuevo, conserva los originales y deja `SIGNING-INCOMPLETE.txt` si falla alguna operación. No distribuir una carpeta con ese marcador.

Usa SHA256 para archivo y sello RFC3161, y verifica firma, publicador y presencia de sello de tiempo. El SHA1 del parámetro CertificateThumbprint identifica el certificado, no el algoritmo de firma. Referencia: [SignTool de Microsoft](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool).

Ejemplo, completando certificado y URL aprobados por la organización:

```powershell
./scripts/sign-windows-release.ps1 -Artifact ./salida/activity-monitor-agent.exe `
  -CertificateThumbprint $thumbprint -TimestampUrl $timestampUrl `
  -OutputDirectory ./salida/ejecutable-firmado
```

Orden de entrega:

1. Compilar desde checkout limpio e identificar commit/versión/destino.
2. Firmar el ejecutable; construir MSI usando ese ejecutable y su nuevo SHA256. No cambiar su versión por renombrado.
3. Firmar el MSI con el mismo procedimiento, usando otro directorio nuevo.
4. Aprovisionar el certificado/publicador confiable en los equipos por un canal independiente. No confiar en un thumbprint enviado únicamente junto al MSI.
5. Generar el manifiesto con `scripts/release-manifest.py` después de firmar; registrar SignTool/SDK, certificado público, timestamp y hashes finales. `signing-receipt.json` registra el resultado por archivo, pero no sustituye un manifiesto firmado ni firma los scripts auxiliares.
6. Ejecutar la matriz final de instalación/actualización/rollback y publicar solo el candidato aceptado. La firma acredita publicador/integridad, no estabilidad ni ausencia de defectos.

El script admite EXE/MSI/DLL. La entrega de asistentes PowerShell requiere el canal de confianza organizacional o firma de scripts; no se afirma que firmar el MSI autentique automáticamente los scripts externos. Tampoco se empaqueta la clave privada.

## Linux

Conservar DEB separados por distribución, arquitectura y versión. Para despliegue general, usar un repositorio APT con metadatos firmados, clave aprovisionada por un canal independiente y `signed-by` limitado a ese repositorio. No usar `trusted=yes` ni desactivar verificación. La infraestructura/clave del repositorio todavía debe configurarse por DevOps.

La herramienta existente `update-signed.sh` verifica firmas OpenSSL de **ejecutables**, no firmas de repositorios APT ni DEB. No intercambiar esos formatos ni presentar SHA256 por sí solo como autenticidad del publicador.

## Pendientes explícitos

Certificado/clave organizacionales, receptor de sellado aprobado, confianza distribuida, firma de scripts/canal de entrega, repositorio APT, manifiestos finales y aceptación de paquetes. No sustituir los candidatos privados existentes por una versión distinta bajo el mismo identificador entregado.

## Certificado interno (alternativa, 2026-11-20)

La entrega 0.2.1 se autoriza sin firma por ahora. No se creó ni instaló ningún
certificado. Para equipos administrados, TI puede emitir un certificado de firma
de código desde su CA interna (AD CS), con EKU Code Signing
`1.3.6.1.5.5.7.3.3`, clave privada restringida al firmante y renovación/revocación
planificadas. Distribuir la cadena de la CA y el publicador por GPO/MDM, mediante
un canal independiente del instalador. Esto no confiere confianza pública.

Para un laboratorio sin CA, este ejemplo genera un certificado autofirmado en
la cuenta del firmante y exporta únicamente su parte pública:

```powershell
$cert = New-SelfSignedCertificate -Type CodeSigningCert `
  -Subject 'CN=Monitor Firma Interna' -CertStoreLocation 'Cert:\CurrentUser\My' `
  -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
  -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(1)
Export-Certificate -Cert $cert -FilePath .\monitor-firma-interna.cer
```

TI debe verificar la huella por otro canal y desplegar solamente el `.cer` a los
almacenes de confianza correspondientes (raíz autofirmada y publicadores de
confianza) de los equipos de laboratorio. Nunca distribuir claves privadas/PFX
con el instalador. Para uso organizacional preferir una CA administrada con
revocación. El firmante necesita la cadena de confianza instalada y un servidor
RFC3161 aprobado para utilizar el script de firma de este proyecto. Después,
firmar EXE, construir y firmar MSI, y regenerar hashes/manifiestos; publicar como
una nueva entrega, sin reemplazar silenciosamente los paquetes ya publicados.

Referencias Microsoft: [New-SelfSignedCertificate](https://learn.microsoft.com/en-us/powershell/module/pki/new-selfsignedcertificate)
y [firma de PowerShell](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_signing).
