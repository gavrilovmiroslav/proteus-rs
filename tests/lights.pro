
event PowerOn();
event PowerOff();

actor Lights {
    statemachine {
        initial LightsOff;

        state LightsOff {
            on PowerOn() goto B;
        };

        state LightsOn {
            on PowerOff() goto A;
        }
    };
};

func main() {
    Lights ! PowerOn();
    Lights ! PowerOff();
    Lights ! PowerOn();
}