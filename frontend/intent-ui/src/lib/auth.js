function getToken(provider) {
  if (typeof window === "undefined") return null;
  return localStorage.getItem(`${provider}_token`);
}
export {
  getToken
};
