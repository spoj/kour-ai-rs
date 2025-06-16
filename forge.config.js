export default {
  packagerConfig: {},
  makers: [
    {
      name: "@electron-forge/maker-zip",
    },
    {
      name: "@electron-forge/maker-deb",
      config: {},
      platforms: ["linux"],
    },
    {
      name: '@electron-forge/maker-squirrel',
    }
  ],
  outDir: "dist",
};
