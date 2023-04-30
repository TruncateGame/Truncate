module.exports = function (eleventyConfig) {
    eleventyConfig.addPassthroughCopy("static");

    eleventyConfig.setServerOptions({
        liveReload: true,
        domDiff: false,
        port: 8005,
        watch: [/** TODO: Font asset, tile asset */],
    });
}
