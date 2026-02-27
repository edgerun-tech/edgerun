function IconBase(props) {
  const { size = 20, class: className, viewBox = "0 0 24 24", children } = props;
  return <svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox={viewBox}
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class={className}
  >
    {children}
  </svg>;
}

const cloud = (props) => <IconBase {...props}><path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" /></IconBase>;
const close = (props) => <IconBase {...props}><path d="M18 6 6 18" /><path d="M6 6 18 18" /></IconBase>;
const sparkles = (props) => <IconBase {...props}><path d="M12 3 14 8l5 2-5 2-2 5-2-5-5-2 5-2z" /></IconBase>;
const chevronRight = (props) => <IconBase {...props}><path d="m9 6 6 6-6 6" /></IconBase>;
const check = (props) => <IconBase {...props}><path d="m20 6-11 11-5-5" /></IconBase>;
const github = (props) => <IconBase {...props}><path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.9a3.4 3.4 0 0 0-.9-2.6c3-.3 6.2-1.5 6.2-6.7A5.2 5.2 0 0 0 20 4.8 4.8 4.8 0 0 0 20 1s-1.1-.3-3.8 1.5a13.4 13.4 0 0 0-7 0C6.5.7 5.4 1 5.4 1a4.8 4.8 0 0 0 0 3.8A5.2 5.2 0 0 0 4 8.8c0 5.2 3.2 6.4 6.2 6.7a3.4 3.4 0 0 0-1 2.6V22" /></IconBase>;
const terminal = (props) => <IconBase {...props}><path d="m4 17 6-6-6-6" /><path d="M12 19h8" /></IconBase>;
const brain = (props) => <IconBase {...props}><path d="M9.5 2a3.5 3.5 0 0 0-3.5 3.5V8A3 3 0 0 0 3 11v2a3 3 0 0 0 3 3h.5A2.5 2.5 0 0 0 9 18.5V22" /><path d="M14.5 2a3.5 3.5 0 0 1 3.5 3.5V8a3 3 0 0 1 3 3v2a3 3 0 0 1-3 3h-.5a2.5 2.5 0 0 1-2.5 2.5V22" /></IconBase>;
const code = (props) => <IconBase {...props}><path d="m16 18 6-6-6-6" /><path d="m8 6-6 6 6 6" /></IconBase>;
const server = (props) => <IconBase {...props}><rect width="20" height="8" x="2" y="3" rx="2" /><rect width="20" height="8" x="2" y="13" rx="2" /><path d="M6 7h.01M6 17h.01" /></IconBase>;
const lock = (props) => <IconBase {...props}><rect width="18" height="11" x="3" y="11" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></IconBase>;
const icons = {
  cloud,
  close,
  sparkles,
  chevronRight,
  check,
  github,
  terminal,
  brain,
  code,
  server,
  lock
};
export {
  icons
};
