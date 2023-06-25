module.exports = function () {
    return {
        environment: process.env.TR_COMMIT || "local"
    };
};