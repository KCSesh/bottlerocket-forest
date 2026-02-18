#ifndef ENGINE_H
#define ENGINE_H

/**
 * Engine telemetry data structure.
 *
 * Contains rpm, temperature, and oil pressure readings
 * from onboard sensors.
 */
struct EngineTelemetry {
    int rpm;
    float coolant_temperature;
    float oil_pressure;
};

/**
 * Retrieves current engine telemetry.
 *
 * Polls sensors and returns aggregated telemetry data.
 */
struct EngineTelemetry get_engine_telemetry(void);

#endif
