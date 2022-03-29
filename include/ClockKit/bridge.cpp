#include "SystemClock.h"
#include "ClockClient.h"
#include "Timestamp.h"
#include "bridge.h"
#include "clockkit/src/main.rs.h"
#include "PhaseLockedClock.h"
#include "kissnet.hpp"
#include <memory>
#include <random>
#include <utility>

std::pair<dex::PhaseLockedClock*, dex::ClockClient*> buildClock(ConfigReader config)
{
    auto cli = new dex::ClockClient(kissnet::endpoint(std::string(config.server), config.port));
    cli->setTimeout(config.timeout);
    cli->setAcknowledge(true);
    auto plc = new dex::PhaseLockedClock(dex::SystemClock::instance(), *cli);
    plc->setPhasePanic(dex::DurFromUsec(config.phasePanic));
    plc->setUpdatePanic(dex::DurFromUsec(config.updatePanic));
    return std::make_pair(plc, cli);
}

std::shared_ptr<dex::PhaseLockedClock> buildPLC(ConfigReader config) {
    auto [plc, cli] = buildClock(config);
    return std::shared_ptr<dex::PhaseLockedClock>(plc);
}

int64_t getValue(std::shared_ptr<dex::PhaseLockedClock> clock) {
    auto val = clock->getValue();
    return dex::UsecFromTp(val);
}


void setPhasePanic(std::shared_ptr<dex::PhaseLockedClock> clock, int64_t micros) {
    auto val = dex::DurFromUsec(micros);
    clock->setPhasePanic(val);
}

void setUpdatePanic(std::shared_ptr<dex::PhaseLockedClock> clock, int64_t micros) {
    auto val = dex::DurFromUsec(micros);
    clock->setUpdatePanic(val);
}

