
event PowerOn(int);
event PowerOff();

actor Lights {
    entry {};
    on PowerOn(x) if (x == 5) {};
    on PowerOn(x) if (x != 5) {};
    statemachine {
        initial B;
        entry { x = 7; x = x + 2; };
        state A {
            on PowerOn(x) goto B {}
        };

        state B {
            on PowerOff() {}
        };
    };
}