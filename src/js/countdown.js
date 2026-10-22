import { listen } from './bridge.js';

const num = document.getElementById('num');

listen('countdown-tick', ({ n }) => {
  if (n <= 0) return;
  num.textContent = n;
  num.classList.remove('pop');
  void num.offsetWidth;
  num.classList.add('pop');
});
