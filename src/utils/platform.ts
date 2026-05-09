export const detectPlatform = (): string => {
  const userAgent = window.navigator.userAgent.toLowerCase();
  if (userAgent.includes("win")) return "windows";
  if (userAgent.includes("mac")) return "darwin";
  return "linux";
};
