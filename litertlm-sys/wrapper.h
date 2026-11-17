// Umbrella header for LiteRT-LM's official C API prebuilt package
// (C API version 1.0.0). Every public header is included here so bindgen
// sees the complete API surface:
//
//   engine.h            Engine / Session / settings / sampler / tokenizer APIs
//                       (also pulls in api_export.h and error_reporter.h)
//   conversation.h      Conversation APIs (also pulls in engine.h)
//   embedding_engine.h  Embedding engine APIs
//   model_info.h        Model metadata APIs
//   experimental.h      Experimental APIs (no stability guarantees)
#include "api_export.h"
#include "error_reporter.h"
#include "engine.h"
#include "conversation.h"
#include "embedding_engine.h"
#include "model_info.h"
#include "experimental.h"
