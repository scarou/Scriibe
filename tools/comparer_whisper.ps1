# Comparer des modèles Whisper : lance un whisper-server par modèle (ports 8191, 8192…), sert la page tools\comparer_whisper.html sur
# http://127.0.0.1:8199/ et l'ouvre dans le navigateur. Le bouton « Arrêter les serveurs » de la page (ou Ctrl+C) arrête tout.
# N'utilise pas le port 8178 de Scriibe : la vraie Scriibe peut rester ouverte. Rien n'est téléchargé.
param(
    [string[]]$Models = @('ggml-base.bin', 'ggml-small.bin'),
    [int]$FirstPort = 8191,
    [int]$PagePort = 8199,
    [switch]$NoBrowser
)
$ErrorActionPreference = 'Stop'

$server = Join-Path $env:LOCALAPPDATA 'Scriibe\engines\whisper\whisper-server.exe'
$folder = Join-Path $env:APPDATA 'Scriibe\models'
$page = Join-Path $PSScriptRoot 'comparer_whisper.html'
if (-not (Test-Path $server)) { throw "whisper-server introuvable : $server" }
if (-not (Test-Path $page)) { throw "page introuvable : $page" }

$children = @()
$listener = $null
try {
    $port = $FirstPort
    foreach ($name in $Models) {
        $model = Join-Path $folder $name
        if (-not (Test-Path $model)) { throw "modèle introuvable : $model" }
        $children += Start-Process -FilePath $server -WindowStyle Hidden -PassThru -ArgumentList @('-m', "`"$model`"", '--host', '127.0.0.1', '--port', $port, '-l', 'auto', '-t', '4')
        Write-Host "$name sur le port $port"
        $port++
    }

    $listener = [System.Net.HttpListener]::new()
    $listener.Prefixes.Add("http://127.0.0.1:$PagePort/")
    $listener.Start()
    if (-not $NoBrowser) { Start-Process "http://127.0.0.1:$PagePort/" }
    Write-Host "Page : http://127.0.0.1:$PagePort/ (bouton « Arrêter les serveurs », ou Ctrl+C)"

    $running = $true
    while ($running) {
        $context = $listener.GetContext()
        $request = $context.Request
        $response = $context.Response
        if ($request.HttpMethod -eq 'POST' -and $request.Url.AbsolutePath -eq '/fin') {
            $running = $false
            $bytes = [Text.Encoding]::UTF8.GetBytes('ok')
        } elseif ($request.Url.AbsolutePath -eq '/') {
            $response.ContentType = 'text/html; charset=utf-8'
            $bytes = [IO.File]::ReadAllBytes($page)
        } else {
            $response.StatusCode = 404
            $bytes = @()
        }
        $response.ContentLength64 = $bytes.Length
        $response.OutputStream.Write($bytes, 0, $bytes.Length)
        $response.Close()
    }
} finally {
    if ($listener) { $listener.Close() }
    foreach ($child in $children) { if (-not $child.HasExited) { $child.Kill() } }
    Write-Host 'Serveurs arrêtés.'
}
