import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs) {
  return twMerge(clsx(inputs));
}

/**
 * Detects the user's operating system
 * @returns {'windows' | 'macos' | 'linux' | 'unknown'}
 */
export function detectOS() {
  if (typeof window === 'undefined') return 'unknown';
  
  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform?.toLowerCase() || '';
  
  if (userAgent.indexOf('win') !== -1 || platform.indexOf('win') !== -1) {
    return 'windows';
  }
  
  if (userAgent.indexOf('mac') !== -1 || platform.indexOf('mac') !== -1) {
    return 'macos';
  }
  
  if (userAgent.indexOf('linux') !== -1 || platform.indexOf('linux') !== -1) {
    return 'linux';
  }
  
  return 'unknown';
}

/**
 * Gets the appropriate download file extension based on OS
 * @param {string} os - The operating system
 * @returns {string} File extension or descriptor
 */
export function getDownloadType(os) {
  switch (os) {
    case 'windows':
      return 'msi';
    case 'macos':
      return 'dmg';
    case 'linux':
      // For Linux, we'll detect the specific distro if possible
      return detectLinuxDistro();
    default:
      return 'AppImage'; // Fallback for Linux
  }
}

/**
 * Detects Linux distribution for appropriate package type
 * @returns {string} Package type (deb, rpm, or AppImage)
 */
function detectLinuxDistro() {
  if (typeof window === 'undefined') return 'deb';
  
  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform?.toLowerCase() || '';
  
  // Check user agent and platform for distro hints
  const fullInfo = userAgent + ' ' + platform;
  
  // Check for Debian/Ubuntu-based distributions
  if (fullInfo.includes('ubuntu') || 
      fullInfo.includes('debian') || 
      fullInfo.includes('mint') ||
      fullInfo.includes('pop') ||
      fullInfo.includes('elementary')) {
    return 'deb';
  }
  
  // Check for Fedora/RHEL/CentOS/RPM-based distributions
  if (fullInfo.includes('fedora') || 
      fullInfo.includes('rhel') || 
      fullInfo.includes('centos') ||
      fullInfo.includes('red hat') ||
      fullInfo.includes('suse') ||
      fullInfo.includes('opensuse')) {
    return 'rpm';
  }
  
  // For most Linux systems, .deb is more common than .rpm
  // Default to .deb instead of AppImage as it's more user-friendly
  return 'deb';
}
