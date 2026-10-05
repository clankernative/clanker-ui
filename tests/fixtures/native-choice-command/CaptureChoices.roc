import pf.Handler
import pf.Api
import pf.Tx
import pf.Context
import CaptureChoicesTypes
import Errors

CaptureChoices :: [].{
	definition = Api.command({
		handler: Handler.local(handle),
		contract,
		execution: Api.current_state([]),
		verification: { input: verify_input, check: verify_result },
	})

	handle : Context, CaptureChoicesTypes.Input -> Tx(CaptureChoicesTypes.Output)
	handle = |_context, input|
		if allowed(input) {
			Tx.succeed({ enabled: input.enabled, density: input.density, channel_count: input.channels.len(), email_selected: input.channels.any(|channel| channel == "email"), push_selected: input.channels.any(|channel| channel == "push") })
		} else {
			Tx.reject(Errors.invalid_choice)
		}

	allowed : CaptureChoicesTypes.Input -> Bool
	allowed = |input|
		(input.density == "compact" or input.density == "comfortable")
			and input.channels.len() <= 2
			and input.channels.all(|channel| channel == "email" or channel == "push")

	contract = {
		errors: [Errors.invalid_choice],
		title: "Check illustrative preferences",
		usage: {
			purpose: "Validate and echo native choice values in the disposable component proof.",
			use_when: ["Testing the ordinary app-owned command form carrier."],
			avoid_when: ["Saving production preferences or changing real task data."],
			preconditions: [],
			effects: ["Records the illustrative invocation through the Native command journal; business rows are unchanged."],
			result: "The validated illustrative choices. No rows are created or changed.",
		},
		inputs: { enabled: "Illustrative Boolean setting.", density: "Exactly compact or comfortable.", channels: { description: "Zero to two illustrative email/push selections.", each: "One app-validated channel." } },
		outputs: { enabled: "The submitted Boolean.", density: "The selected density.", channel_count: "The selected channel count.", email_selected: "Whether email was selected.", push_selected: "Whether push was selected." },
		example,
		input_sources: |_| [],
		follow_ups: [],
		deprecated: Bool.False,
	}

	example : {} -> Try({ input : CaptureChoicesTypes.Input, output : CaptureChoicesTypes.Output }, Str)
	example = |_| Ok({ input: { enabled: Bool.True, density: "compact", channels: ["email"] }, output: { enabled: Bool.True, density: "compact", channel_count: 1, email_selected: Bool.True, push_selected: Bool.False } })

	verify_input : Str, U64 -> Try(CaptureChoicesTypes.Input, Str)
	verify_input = |_snapshot, _seed| Ok({ enabled: Bool.False, density: "comfortable", channels: [] })

	verify_result : Str, CaptureChoicesTypes.Output, Str -> Try(Bool, Str)
	verify_result = |before, output, after| Ok(before == after and (output.density == "compact" or output.density == "comfortable") and output.channel_count <= 2)

	invalid_input : Str, U64 -> Try(CaptureChoicesTypes.Input, Str)
	invalid_input = |_snapshot, _seed| Ok({ enabled: Bool.False, density: "unsupported", channels: [] })
}
