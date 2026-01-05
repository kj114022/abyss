const fs = require('fs');
const path = require('path');
const https = require('https');
const os = require('os');
const { execSync } = require('child_process');

const VERSION = '1.0.0'; // Must match Cargo.toml
const BIN_NAME = 'abyss';
const REPO = 'kj/abyss';

// Map Node.js platform/arch to Rust targets
const PLATFORM_MAP = {
  'darwin': {
    'x64': 'x86_64-apple-darwin',
    'arm64': 'aarch64-apple-darwin'
  },
  'linux': {
    'x64': 'x86_64-unknown-linux-gnu',
    'arm64': 'aarch64-unknown-linux-gnu'
  },
  'win32': {
    'x64': 'x86_64-pc-windows-msvc',
    'arm64': 'aarch64-pc-windows-msvc'
  }
};

function getTarget() {
  const platform = os.platform();
  const arch = os.arch();
  
  if (!PLATFORM_MAP[platform] || !PLATFORM_MAP[platform][arch]) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }
  
  return PLATFORM_MAP[platform][arch];
}

function install() {
  const target = getTarget();
  const platform = os.platform();
  const ext = platform === 'win32' ? '.exe' : '';
  const archiveExt = platform === 'win32' ? '.zip' : '.tar.gz';
  
  // URL Structure: https://github.com/USER/REPO/releases/download/vVERSION/abyss-vVERSION-TARGET.tar.gz
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${BIN_NAME}-v${VERSION}-${target}${archiveExt}`;
  
  const binDir = path.join(__dirname, '..', 'bin');
  const destPath = path.join(binDir, `${BIN_NAME}${ext}`);

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  console.log(`\x1b[36mDownloading Abyss v${VERSION} for ${target}...\x1b[0m`);
  console.log(`Source: ${url}`);

  // 1. Download to temp file
  const tempFile = path.join(os.tmpdir(), `abyss-${Date.now()}${archiveExt}`);
  const file = fs.createWriteStream(tempFile);

  https.get(url, (response) => {
    if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        // Handle redirect (GitHub Releases always redirect)
        https.get(response.headers.location, (redirectResponse) => {
             pipeResponse(redirectResponse, file, tempFile, destPath, archiveExt);
        });
        return;
    }
    pipeResponse(response, file, tempFile, destPath, archiveExt);
  }).on('error', (err) => {
    console.error('\x1b[31mDownload failed:\x1b[0m', err.message);
    process.exit(1);
  });
}

function pipeResponse(response, file, tempFile, destPath, archiveExt) {
    if (response.statusCode !== 200) {
        console.error(`\x1b[31mFailed to download binary. Status Code: ${response.statusCode}\x1b[0m`);
        process.exit(1);
    }

    response.pipe(file);

    file.on('finish', () => {
        file.close(() => {
            extract(tempFile, destPath, archiveExt);
        });
    });
}

function extract(tempFile, destPath, archiveExt) {
    console.log('\x1b[36mExtracting...\x1b[0m');
    const binDir = path.dirname(destPath);
    
    try {
        if (archiveExt === '.zip') {
             // Basic unzip for windows (requires PowerShell or similar, keeping generic for now)
             // For strict zero-dep unzip in Node, we might need a library, but simplest is system command
             execSync(`tar -xf "${tempFile}" -C "${binDir}"`); // tar handles zip in modern bsdtar/gnu tar
        } else {
             execSync(`tar -xzf "${tempFile}" -C "${binDir}"`);
        }
        
        // Cleanup
        fs.unlinkSync(tempFile);
        
        // Move/Rename if nested? Usually releases are flattened or in a folder. 
        // Assuming release contains generic 'abyss' binary at root of tarball.
        
        // If binary is not at destPath, find it (sometimes artifacts are versioned folder)
        // Check directory contents
        const files = fs.readdirSync(binDir);
        // Find the 'abyss' or 'abyss.exe' executable in generated folder
        // Simplified: Assume strictly packed as binary only.
        
        // Chmod
        if (os.platform() !== 'win32') {
             // In case it extracted to a different name, or just to be safe set all to executable
             // but let's assume 'abyss' exists
             if(fs.existsSync(destPath)) {
                 fs.chmodSync(destPath, 0o755);
             }
        }
        
        console.log('\x1b[32mAbyss installed successfully!\x1b[0m');
    } catch (e) {
        console.error('\x1b[31mExtraction failed:\x1b[0m', e.message);
        process.exit(1);
    }
}

install();
