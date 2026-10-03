const searchInput = document.getElementById('searchInput');
const searchResults = document.getElementById('searchResults');
const themeToggle = document.getElementById('themeToggle');
const navHeaders = document.querySelectorAll('.nav-header');
const mobileMenuButton = document.getElementById('mobileMenuButton');
const sidebar = document.getElementById('sidebar');

const docsIndex = [
  { title: 'Getting Started', url: '#getting-started', keywords: 'welcome introduction boot quickstart myos milestone' },
  { title: 'Project Goals', url: '#project-goals', keywords: 'objectives desktop windows xp inspiration' },
  { title: 'Roadmap', url: '#roadmap', keywords: 'plan milestones future boot graphics desktop filesystem apps' },
  { title: 'System Overview', url: '#system-overview', keywords: 'architecture components boot kernel gui' },
  { title: 'Boot Process', url: '#boot-process', keywords: 'uefi qemu firmware bootloader kernel startup' },
  { title: 'Kernel', url: '#kernel', keywords: 'kernel x86_64 runtime initialization' },
  { title: 'Memory', url: '#memory', keywords: 'physical memory paging allocator heap' },
  { title: 'Filesystem', url: '#filesystem', keywords: 'block device file read write delete remount persistence' },
  { title: 'Graphics', url: '#graphics', keywords: 'framebuffer rendering software gpu display' },
  { title: 'GUI Framework', url: '#gui-framework', keywords: 'canvas painter button rectangle bitmap text widget' },
  { title: 'Window Manager', url: '#window-manager', keywords: 'windows z-order focus resizing menus' },
  { title: 'Desktop', url: '#desktop', keywords: 'wallpaper icons taskbar start menu system tray' },
  { title: 'Taskbar', url: '#taskbar', keywords: 'start button system tray clock running apps' },
  { title: 'Start Menu', url: '#start-menu', keywords: 'start menu programs search shutdown' },
  { title: 'Notepad', url: '#notepad', keywords: 'text editor save open clipboard' },
  { title: 'Explorer', url: '#explorer', keywords: 'file manager folders files navigation' },
  { title: 'Calculator', url: '#calculator', keywords: 'math arithmetic decimals clear' },
  { title: 'Terminal', url: '#terminal', keywords: 'cli shell commands dir type mkdir' },
  { title: 'Settings', url: '#settings', keywords: 'display wallpaper theme mouse keyboard system' },
  { title: 'Building', url: '#building', keywords: 'cargo rust build toolchain target' },
  { title: 'QEMU', url: '#qemu', keywords: 'qemu ovmf emulation test boot' },
  { title: 'Debugging', url: '#debugging', keywords: 'logs panic diagnostics serial' },
  { title: 'Testing', url: '#testing', keywords: 'verification stable boot validation' }
];

const savedTheme = localStorage.getItem('myos-doc-theme') || 'light';
if (savedTheme === 'dark') {
  document.body.classList.add('dark');
  themeToggle.textContent = '☀️';
}

themeToggle.addEventListener('click', () => {
  const isDark = document.body.classList.toggle('dark');
  localStorage.setItem('myos-doc-theme', isDark ? 'dark' : 'light');
  themeToggle.textContent = isDark ? '☀️' : '🌙';
});

navHeaders.forEach((button) => {
  const target = document.getElementById(button.dataset.target);
  button.addEventListener('click', () => {
    target.classList.toggle('open');
    const state = target.classList.contains('open') ? 'open' : 'closed';
    localStorage.setItem('myos-nav-' + button.dataset.target, state);
  });

  const savedState = localStorage.getItem('myos-nav-' + button.dataset.target);
  if (savedState === 'closed') {
    target.classList.remove('open');
  }
});

searchInput.addEventListener('input', () => {
  const query = searchInput.value.trim().toLowerCase();
  if (!query) {
    searchResults.classList.add('hidden');
    searchResults.innerHTML = '';
    return;
  }

  const results = docsIndex.filter((item) => {
    const haystack = `${item.title} ${item.keywords}`.toLowerCase();
    return haystack.includes(query);
  });

  if (!results.length) {
    searchResults.innerHTML = '<div class="search-item">No matches found.</div>';
    searchResults.classList.remove('hidden');
    return;
  }

  searchResults.innerHTML = results
    .slice(0, 8)
    .map(
      (item) =>
        `<a class="search-item" href="${item.url}">${item.title}</a>`
    )
    .join('');
  searchResults.classList.remove('hidden');
});

document.addEventListener('keydown', (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    searchInput.focus();
  }

  if (event.key === 'Escape') {
    searchResults.classList.add('hidden');
    searchInput.blur();
  }
});

mobileMenuButton.addEventListener('click', () => {
  sidebar.classList.toggle('mobile-open');
  if (sidebar.style.display === 'none') {
    sidebar.style.display = 'block';
  } else if (window.innerWidth <= 900) {
    sidebar.style.display = sidebar.style.display === 'block' ? 'none' : 'block';
  }
});

const currentHash = window.location.hash || '#getting-started';
const currentLinks = document.querySelectorAll('.nav-items a');
currentLinks.forEach((link) => {
  if (link.getAttribute('href') === currentHash) {
    link.classList.add('current');
  }
});

window.addEventListener('resize', () => {
  if (window.innerWidth > 900) {
    sidebar.style.display = 'block';
  }
});
